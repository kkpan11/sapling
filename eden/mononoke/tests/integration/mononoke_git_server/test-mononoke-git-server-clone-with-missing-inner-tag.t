# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License found in the LICENSE file in the root
# directory of this source tree.

  $ . "${TEST_FIXTURES}/library.sh"
  $ REPOTYPE="blob_files"
  $ setup_common_config $REPOTYPE
  $ GIT_REPO_ORIGIN="${TESTTMP}/origin/repo-git"

# Setup git repository
  $ mkdir -p "$GIT_REPO_ORIGIN"
  $ cd "$GIT_REPO_ORIGIN"
  $ git init -q
  $ echo "this is file1" > file1
  $ git add file1
  $ git commit -qam "Add file1"
  $ git tag -a -m "new tag" first_tag
  $ echo "this is file2" > file2
  $ git add file2
  $ git commit -qam "Add file2"
  $ git tag -a empty_tag -m ""
# Create another tag that points to first_tag
  $ git config advice.nestedTag false
  $ git tag -a -m "nested tag" second_tag first_tag
# Now remove the first_tag from the repository
  $ git tag --delete first_tag
  Deleted tag 'first_tag' (was 8963e1f)

# Capture all the known Git objects from the repo
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/object_list

# Import it into Mononoke
  $ cd "$TESTTMP"
  $ quiet gitimport "$GIT_REPO_ORIGIN" --derive-hg --generate-bookmarks full-repo

  $ cat $TESTTMP/object_list | grep first_tag
  8963e1f55d1346a07c3aec8c8fc72bf87d0452b1 tag first_tag
  $ ls $TESTTMP/blobstore/blobs | grep 8963e1f55d1346a07c3aec8c8fc72bf87d0452b1
  blob-repo0000.git_object.8963e1f55d1346a07c3aec8c8fc72bf87d0452b1
  blob-repo0000.git_packfile_base_item.8963e1f55d1346a07c3aec8c8fc72bf87d0452b1

# Start up the Mononoke Git Service
  $ mononoke_git_service
# Clone the Git repo from Mononoke
  $ quiet git_client clone $MONONOKE_GIT_SERVICE_BASE_URL/$REPONAME.git
# Verify that we get the same Git repo back that we started with
  $ cd $REPONAME  
  $ git rev-list --objects --all | git cat-file --batch-check='%(objectname) %(objecttype) %(rest)' | sort > $TESTTMP/new_object_list
  $ diff -w $TESTTMP/new_object_list $TESTTMP/object_list 
