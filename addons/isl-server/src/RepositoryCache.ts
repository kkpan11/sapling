/**
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under the MIT license found in the
 * LICENSE file in the root directory of this source tree.
 */

import type {AbsolutePath, RepositoryError, ValidatedRepoInfo} from 'isl/src/types';
import type {RepositoryContext} from './serverTypes';

import {TypedEventEmitter} from 'shared/TypedEventEmitter';
import {ensureTrailingPathSep} from 'shared/pathUtils';
import {Repository} from './Repository';

/**
 * Reference-counting access to a {@link Repository}, via a Promise.
 * Be sure to `unref()` when no longer needed.
 */
export interface RepositoryReference {
  promise: Promise<Repository | RepositoryError>;
  unref: () => unknown;
}

/**
 * We return `RepositoryReference`s synchronously before we have the Repository,
 * but reference counts should be associated with the actual async constructed Repository,
 * which is why we can't return RefCounted<Repository> directly.
 */
class RepositoryReferenceImpl implements RepositoryReference {
  constructor(
    public promise: Promise<Repository | RepositoryError>,
    private disposeFunc: () => void,
  ) {}
  public unref() {
    if (!this.disposed) {
      this.disposed = true;
      this.disposeFunc();
    }
  }

  internalReference: RefCounted<Repository> | undefined;
  disposed = false;
}

class RefCounted<T extends {dispose: () => void}> {
  constructor(public value: T) {}
  private references = 0;

  public isDisposed = false;
  public ref() {
    this.references++;
  }
  public getNumberOfReferences() {
    return this.references;
  }
  public dispose() {
    this.references--;
    if (!this.isDisposed && this.references === 0) {
      this.isDisposed = true;
      this.value.dispose();
    }
  }
}

/**
 * Allow reusing Repository instances by storing instances by path,
 * and controlling how Repositories are created.
 *
 * Some async work is needed to construct repositories (finding repo root dir),
 * so it's possible to duplicate some work if multiple repos are constructed at similar times.
 * We still enable Repository reuse in this case by double checking for pre-existing Repos at the last second.
 */
class RepositoryCache {
  // allow mocking Repository in tests
  constructor(private RepositoryType = Repository) {}

  /**
   * Previously distributed RepositoryReferences, keyed by repository root path
   * Note that Repositories do not define their own `cwd`, and can be reused across cwds.
   */
  private reposByRoot = new Map<AbsolutePath, RefCounted<Repository>>();
  private activeReposEmitter = new TypedEventEmitter<'change', undefined>();

  private lookup(dirGuess: AbsolutePath): RefCounted<Repository> | undefined {
    for (const repo of this.reposByRoot.values()) {
      if (
        dirGuess === repo.value.info.repoRoot ||
        dirGuess.startsWith(ensureTrailingPathSep(repo.value.info.repoRoot))
      ) {
        if (!repo.isDisposed) {
          return repo;
        }
      }
    }
    return undefined;
  }

  /**
   * Create a new Repository, or reuse if one already exists.
   * Repositories are reference-counted to ensure they can be disposed when no longer needed.
   */
  getOrCreate(ctx: RepositoryContext): RepositoryReference {
    // Fast path: if this cwd is already a known repo root, we can use it directly.
    // This only works if the cwd happens to be the repo root.
    const found = this.lookup(ctx.cwd);
    if (found) {
      found.ref();
      return new RepositoryReferenceImpl(Promise.resolve(found.value), () => found.dispose());
    }

    // More typically, we can reuse a Repository among different cwds:

    // eslint-disable-next-line prefer-const
    let ref: RepositoryReferenceImpl;
    const lookupRepoInfoAndReuseIfPossible = async (): Promise<Repository | RepositoryError> => {
      // TODO: we should rate limit how many getRepoInfos we run at a time, and make other callers just wait.
      // this would guard against querying lots of redundant paths within the same repo.
      // This is probably not necessary right now, but would be useful for a VS Code extension where we need to query
      // individual file paths to add diff gutters.
      const repoInfo = await this.RepositoryType.getRepoInfo(ctx);
      // important: there should be no `await` points after here, to ensure there is no race when reusing Repositories.
      if (repoInfo.type !== 'success') {
        // No repository found at this root, or some other error prevents the repo from being created
        return repoInfo;
      }

      if (ref.disposed) {
        // If the reference is disposed, the caller gave up while waiting for the repo to be created.
        // make sure we don't create a dangling Repository.
        return {type: 'unknownError', error: new Error('Repository already disposed')};
      }

      // Now that we've spent some async time to determine this repo's actual root,
      // we can check if we already have a reference to it saved.
      // This way, we can still reuse a Repository, and only risk duplicating `getRepoInfo` work.
      const newlyFound = this.lookup(repoInfo.repoRoot);
      if (newlyFound) {
        // if it is found now, it means either the cwd differs from the repo root (lookup fails), or
        // another instance was created at the same time and finished faster than this one (lookup failed before, but succeeds now).

        newlyFound.ref();
        ref.internalReference = newlyFound;
        return newlyFound.value;
      }

      // This is where we actually start new subscriptions and trigger work, so we should only do this
      // once we're sure we don't have a repository to reuse.
      const repo = new this.RepositoryType(
        repoInfo as ValidatedRepoInfo, // repoInfo is now guaranteed to have these root/dotdir set
        ctx,
      );

      const internalRef = new RefCounted(repo);
      internalRef.ref();
      ref.internalReference = internalRef;
      this.reposByRoot.set(repoInfo.repoRoot, internalRef);
      this.activeReposEmitter.emit('change');
      return repo;
    };
    ref = new RepositoryReferenceImpl(lookupRepoInfoAndReuseIfPossible(), () => {
      if (ref.internalReference) {
        ref.internalReference.dispose();
      }
      ref.unref();
    });
    return ref;
  }

  /**
   * Lookup a cached repository without creating a new one if it doesn't exist
   */
  public cachedRepositoryForPath(path: AbsolutePath): Repository | undefined {
    const ref = this.lookup(path);
    return ref?.value;
  }

  public onChangeActiveRepos(cb: (repos: Array<Repository>) => unknown): () => unknown {
    const onChange = () => {
      cb([...this.reposByRoot.values()].map(ref => ref.value));
    };
    this.activeReposEmitter.on('change', onChange);
    // start with initial repos set
    onChange();
    return () => this.activeReposEmitter.off('change', onChange);
  }

  /** Clean up all known repos. Mostly useful for testing. */
  clearCache() {
    this.reposByRoot.forEach(value => value.dispose());
    this.reposByRoot = new Map();
    this.activeReposEmitter.removeAllListeners();
  }

  public numberOfActiveServers(): number {
    let numActive = 0;
    for (const repo of this.reposByRoot.values()) {
      numActive += repo.getNumberOfReferences();
    }
    return numActive;
  }
}

export const __TEST__ = {RepositoryCache};

export const repositoryCache = new RepositoryCache();
