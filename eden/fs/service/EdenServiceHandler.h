/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License version 2.
 */

#pragma once

#include <fb303/BaseService.h>
#include <optional>
#include "eden/common/os/ProcessId.h"
#include "eden/common/telemetry/TraceBus.h"
#include "eden/common/utils/PathFuncs.h"
#include "eden/common/utils/RefPtr.h"
#include "eden/fs/eden-config.h"
#include "eden/fs/inodes/EdenMountHandle.h"
#include "eden/fs/inodes/InodePtrFwd.h"
#include "eden/fs/service/gen-cpp2/StreamingEdenService.h"
#include "eden/fs/telemetry/ActivityBuffer.h"

namespace folly {
template <typename T>
class Future;
}

namespace {
class PrefetchFetchContext;
} // namespace

namespace facebook::eden {

class Hash20;
class Hash32;
class BlobAuxData;
class EdenMount;
class EdenServer;
class TreeInode;
class ObjectFetchContext;
using ObjectFetchContextPtr = RefPtr<ObjectFetchContext>;
struct EntryAttributes;
struct EntryAttributeFlags;
template <typename T>
class ImmediateFuture;
class UsageService;

extern const char* const kServiceName;

const int EXPENSIVE_GLOB_FILES_DURATION = 5;

struct ThriftRequestTraceEvent : TraceEventBase {
  enum Type : unsigned char {
    START,
    FINISH,
  };

  ThriftRequestTraceEvent() = delete;

  static ThriftRequestTraceEvent start(
      uint64_t requestId,
      folly::StringPiece method,
      OptionalProcessId clientPid);

  static ThriftRequestTraceEvent finish(
      uint64_t requestId,
      folly::StringPiece method,
      OptionalProcessId clientPid);

  ThriftRequestTraceEvent(
      Type type,
      uint64_t requestId,
      folly::StringPiece method,
      OptionalProcessId clientPid)
      : type(type),
        requestId(requestId),
        method(method),
        clientPid(clientPid) {}

  Type type;
  uint64_t requestId;
  // Safe to use StringPiece because method names are string literals.
  folly::StringPiece method;
  OptionalProcessId clientPid;
};

/*
 * Handler for the EdenService thrift interface
 */
class EdenServiceHandler : virtual public StreamingEdenServiceSvIf,
                           public fb303::BaseService {
 public:
  explicit EdenServiceHandler(
      std::vector<std::string> originalCommandLine,
      EdenServer* server,
      std::unique_ptr<UsageService> usageService);
  ~EdenServiceHandler() override;

  EdenServiceHandler(EdenServiceHandler const&) = delete;
  EdenServiceHandler& operator=(EdenServiceHandler const&) = delete;

  /**
   * Return a newly initialized ActivityBuffer<ThriftRequestTraceEvent> if
   * using ActivityBuffers is enabled and return std::nullopt otherwise.
   */
  std::optional<ActivityBuffer<ThriftRequestTraceEvent>>
  initThriftRequestActivityBuffer();

  std::unique_ptr<apache::thrift::AsyncProcessor> getProcessor() override;

  folly::SemiFuture<folly::Unit> semifuture_mount(
      std::unique_ptr<MountArgument> mount) override;

  folly::SemiFuture<folly::Unit> semifuture_unmount(
      std::unique_ptr<std::string> mountPoint) override;

  folly::SemiFuture<folly::Unit> semifuture_unmountV2(
      std::unique_ptr<UnmountArgument> unmountArg) override;

  void listMounts(std::vector<MountInfo>& results) override;

  folly::SemiFuture<std::unique_ptr<std::vector<CheckoutConflict>>>
  semifuture_checkOutRevision(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> hash,
      CheckoutMode checkoutMode,
      std::unique_ptr<CheckOutRevisionParams> params) override;

  folly::SemiFuture<folly::Unit> semifuture_resetParentCommits(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<WorkingDirectoryParents> parents,
      std::unique_ptr<ResetParentCommitsParams> params) override;

  void getCurrentSnapshotInfo(
      GetCurrentSnapshotInfoResponse& out,
      std::unique_ptr<GetCurrentSnapshotInfoRequest> params) override;

  folly::SemiFuture<folly::Unit> semifuture_synchronizeWorkingCopy(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<SynchronizeWorkingCopyParams> params) override;

  folly::SemiFuture<folly::Unit> semifuture_addBindMount(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> repoPath,
      std::unique_ptr<std::string> targetPath) override;
  folly::SemiFuture<folly::Unit> semifuture_removeBindMount(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> repoPath) override;

  folly::SemiFuture<std::unique_ptr<std::vector<SHA1Result>>>
  semifuture_getSHA1(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::vector<std::string>> paths,
      std::unique_ptr<SyncBehavior> sync) override;

  folly::SemiFuture<std::unique_ptr<std::vector<Blake3Result>>>
  semifuture_getBlake3(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::vector<std::string>> paths,
      std::unique_ptr<SyncBehavior> sync) override;

  folly::SemiFuture<std::unique_ptr<std::vector<DigestHashResult>>>
  semifuture_getDigestHash(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::vector<std::string>> paths,
      std::unique_ptr<SyncBehavior> sync) override;

  void getCurrentJournalPosition(
      JournalPosition& out,
      std::unique_ptr<std::string> mountPoint) override;

  void getFilesChangedSince(
      FileDelta& out,
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<JournalPosition> fromPosition) override;

  void setJournalMemoryLimit(
      std::unique_ptr<PathString> mountPoint,
      int64_t limit) override;

  int64_t getJournalMemoryLimit(
      std::unique_ptr<PathString> mountPoint) override;

  void flushJournal(std::unique_ptr<PathString> mountPoint) override;

  void debugGetRawJournal(
      DebugGetRawJournalResponse& out,
      std::unique_ptr<DebugGetRawJournalParams> params) override;

  folly::SemiFuture<std::unique_ptr<std::vector<EntryInformationOrError>>>
  semifuture_getEntryInformation(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::vector<std::string>> paths,
      std::unique_ptr<SyncBehavior> sync) override;

  folly::SemiFuture<std::unique_ptr<std::vector<FileInformationOrError>>>
  semifuture_getFileInformation(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::vector<std::string>> paths,
      std::unique_ptr<SyncBehavior> sync) override;

  folly::SemiFuture<std::unique_ptr<GetAttributesFromFilesResult>>
  semifuture_getAttributesFromFiles(
      std::unique_ptr<GetAttributesFromFilesParams> params) override;

  folly::SemiFuture<std::unique_ptr<GetAttributesFromFilesResultV2>>
  semifuture_getAttributesFromFilesV2(
      std::unique_ptr<GetAttributesFromFilesParams> params) override;

  folly::SemiFuture<std::unique_ptr<ReaddirResult>> semifuture_readdir(
      std::unique_ptr<ReaddirParams> params) override;

  folly::SemiFuture<std::unique_ptr<Glob>> semifuture_globFiles(
      std::unique_ptr<GlobParams> params) override;

  folly::SemiFuture<folly::Unit> semifuture_prefetchFiles(
      std::unique_ptr<PrefetchParams> params) override;

  folly::SemiFuture<std::unique_ptr<PrefetchResult>> semifuture_prefetchFilesV2(
      std::unique_ptr<PrefetchParams> params) override;

  folly::SemiFuture<std::unique_ptr<Glob>> semifuture_predictiveGlobFiles(
      std::unique_ptr<GlobParams> params) override;

  folly::SemiFuture<folly::Unit> semifuture_chown(
      std::unique_ptr<std::string> mountPoint,
      int32_t uid,
      int32_t gid) override;

  folly::SemiFuture<std::unique_ptr<ChangeOwnershipResponse>>
  semifuture_changeOwnership(
      std::unique_ptr<ChangeOwnershipRequest> request) override;

  apache::thrift::ServerStream<JournalPosition> subscribeStreamTemporary(
      std::unique_ptr<std::string> mountPoint) override;

  apache::thrift::ServerStream<JournalPosition> streamJournalChanged(
      std::unique_ptr<std::string> mountPoint) override;

  apache::thrift::ServerStream<FsEvent> traceFsEvents(
      std::unique_ptr<std::string> mountPoint,
      int64_t eventCategoryMask) override;

  apache::thrift::ServerStream<ThriftRequestEvent> traceThriftRequestEvents()
      override;

  apache::thrift::ServerStream<HgEvent> traceHgEvents(
      std::unique_ptr<std::string> mountPoint) override;

  apache::thrift::ServerStream<InodeEvent> traceInodeEvents(
      std::unique_ptr<std::string> mountPoint) override;

  apache::thrift::ServerStream<TaskEvent> traceTaskEvents(
      std::unique_ptr<::facebook::eden::TraceTaskEventsRequest> request)
      override;

  folly::SemiFuture<std::unique_ptr<GetScmStatusResult>>
  semifuture_getScmStatusV2(
      std::unique_ptr<GetScmStatusParams> params) override;

  apache::thrift::ResponseAndServerStream<ChangesSinceResult, ChangedFileResult>
  streamChangesSince(std::unique_ptr<StreamChangesSinceParams> params) override;

  void sync_changesSinceV2(
      ChangesSinceV2Result& result,
      std::unique_ptr<ChangesSinceV2Params> params) override;

  apache::thrift::ResponseAndServerStream<ChangesSinceResult, ChangedFileResult>
  streamSelectedChangesSince(
      std::unique_ptr<StreamSelectedChangesSinceParams> params) override;

  folly::SemiFuture<std::unique_ptr<ScmStatus>> semifuture_getScmStatus(
      std::unique_ptr<std::string> mountPoint,
      bool listIgnored,
      std::unique_ptr<std::string> commitHash) override;

  folly::SemiFuture<std::unique_ptr<ScmStatus>>
  semifuture_getScmStatusBetweenRevisions(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> oldHash,
      std::unique_ptr<std::string> newHash) override;

  folly::SemiFuture<std::unique_ptr<MatchFileSystemResponse>>
  semifuture_matchFilesystem(
      std::unique_ptr<MatchFileSystemRequest> params) override;

  void debugGetScmTree(
      std::vector<ScmTreeEntry>& entries,
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> id,
      bool localStoreOnly) override;

  folly::SemiFuture<std::unique_ptr<DebugGetScmBlobResponse>>
  semifuture_debugGetBlob(
      std::unique_ptr<DebugGetScmBlobRequest> request) override;

  folly::SemiFuture<std::unique_ptr<DebugGetBlobMetadataResponse>>
  semifuture_debugGetBlobMetadata(
      std::unique_ptr<DebugGetBlobMetadataRequest> request) override;

  folly::SemiFuture<std::unique_ptr<DebugGetScmTreeResponse>>
  semifuture_debugGetTree(
      std::unique_ptr<DebugGetScmTreeRequest> request) override;

  void debugInodeStatus(
      std::vector<TreeInodeDebugInfo>& inodeInfo,
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> path,
      int64_t flags,
      std::unique_ptr<SyncBehavior> sync) override;

  void debugOutstandingFuseCalls(
      std::vector<FuseCall>& outstandingCalls,
      std::unique_ptr<std::string> mountPoint) override;

  void debugOutstandingNfsCalls(
      std::vector<NfsCall>& outstandingCalls,
      std::unique_ptr<std::string> mountPoint) override;

  void debugOutstandingPrjfsCalls(
      std::vector<PrjfsCall>& outstandingCalls,
      std::unique_ptr<std::string> mountPoint) override;

  void debugOutstandingThriftRequests(
      std::vector<ThriftRequestMetadata>& outstandingCalls) override;

  void debugOutstandingHgEvents(
      std::vector<HgEvent>& outstandingEvents,
      std::unique_ptr<std::string> mountPoint) override;

  void debugStartRecordingActivity(
      ActivityRecorderResult& result,
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> outputPath) override;

  void debugStopRecordingActivity(
      ActivityRecorderResult& result,
      std::unique_ptr<std::string> mountPoint,
      int64_t unique) override;

  void debugListActivityRecordings(
      ListActivityRecordingsResult& result,
      std::unique_ptr<std::string> mountPoint) override;

  void debugGetInodePath(
      InodePathDebugInfo& inodePath,
      std::unique_ptr<std::string> mountPoint,
      int64_t inodeNumber) override;

  void clearFetchCounts() override;

  void clearFetchCountsByMount(std::unique_ptr<std::string> mountPath) override;

  void getAccessCounts(GetAccessCountsResult& result, int64_t duration)
      override;

  void clearAndCompactLocalStore() override;

  void debugClearLocalStoreCaches() override;

  void debugCompactLocalStorage() override;

  int64_t debugDropAllPendingRequests() override;

  int64_t unloadInodeForPath(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> path,
      std::unique_ptr<TimeSpec> age) override;

  folly::SemiFuture<std::unique_ptr<DebugInvalidateResponse>>
  semifuture_debugInvalidateNonMaterialized(
      std::unique_ptr<DebugInvalidateRequest> params) override;

  void flushStatsNow() override;

  folly::SemiFuture<folly::Unit> semifuture_invalidateKernelInodeCache(
      std::unique_ptr<std::string> mountPoint,
      std::unique_ptr<std::string> path) override;

  void getStatInfo(
      InternalStats& result,
      std::unique_ptr<GetStatInfoParams> params) override;

  void enableTracing() override;
  void disableTracing() override;
  void getTracePoints(std::vector<TracePoint>& result) override;

  void getRetroactiveThriftRequestEvents(
      GetRetroactiveThriftRequestEventsResult& result) override;

  void getRetroactiveHgEvents(
      GetRetroactiveHgEventsResult& result,
      std::unique_ptr<GetRetroactiveHgEventsParams> params) override;

  void getRetroactiveInodeEvents(
      GetRetroactiveInodeEventsResult& result,
      std::unique_ptr<GetRetroactiveInodeEventsParams> params) override;

  void injectFault(std::unique_ptr<FaultDefinition> fault) override;
  bool removeFault(std::unique_ptr<RemoveFaultArg> fault) override;
  int64_t unblockFault(std::unique_ptr<UnblockFaultArg> info) override;
  void getBlockedFaults(
      GetBlockedFaultsResponse& out,
      std::unique_ptr<GetBlockedFaultsRequest> request) override;

  folly::SemiFuture<std::unique_ptr<SetPathObjectIdResult>>
  semifuture_setPathObjectId(
      std::unique_ptr<SetPathObjectIdParams> params) override;

  folly::SemiFuture<folly::Unit> semifuture_removeRecursively(
      std::unique_ptr<RemoveRecursivelyParams> params) override;

  folly::SemiFuture<folly::Unit> semifuture_ensureMaterialized(
      std::unique_ptr<EnsureMaterializedParams> params) override;

  void reloadConfig() override;

  void getDaemonInfo(DaemonInfo& result) override;

  apache::thrift::ResponseAndServerStream<DaemonInfo, std::string>
  streamStartStatus() override;
  /**
   * Checks the PrivHelper connection.
   * For Windows, result.connected will always be set to true.
   */
  void checkPrivHelper(PrivHelperInfo& result) override;

  int64_t getPid() override;

  void getCheckoutProgressInfo(
      CheckoutProgressInfoResponse& ret,
      std::unique_ptr<CheckoutProgressInfoRequest> params) override;

  /**
   * A thrift client has requested that we shutdown.
   */
  void initiateShutdown(std::unique_ptr<std::string> reason) override;

  void getConfig(
      EdenConfigData& result,
      std::unique_ptr<GetConfigParams> params) override;

  /**
   * Enable all backing stores to record fetched files
   */
  void startRecordingBackingStoreFetch() override;

  /**
   * Make all backing stores stop recording
   * fetched files. Previous records for different kinds of backing
   * stores will be returned by backing store types.
   */
  void stopRecordingBackingStoreFetch(GetFetchedFilesResult& results) override;

  /**
   * Returns the pid that caused the Thrift request running on the calling
   * Thrift worker thread and registers it with the ProcessInfoCache.
   *
   * This must be run from a Thrift worker thread, because the calling pid is
   * stored in a thread local variable.
   */
  OptionalProcessId getAndRegisterClientPid();

  folly::SemiFuture<std::unique_ptr<StartFileAccessMonitorResult>>
  semifuture_startFileAccessMonitor(
      std::unique_ptr<StartFileAccessMonitorParams> params) override;

  folly::SemiFuture<std::unique_ptr<StopFileAccessMonitorResult>>
  semifuture_stopFileAccessMonitor() override;

  void sendNotification(
      SendNotificationResponse& response,
      std::unique_ptr<SendNotificationRequest> request) override;

  void listRedirections(
      ListRedirectionsResponse& response,
      std::unique_ptr<ListRedirectionsRequest> request) override;

  folly::SemiFuture<std::unique_ptr<GetFileContentResponse>>
  semifuture_getFileContent(
      std::unique_ptr<GetFileContentRequest> request) override;

 private:
  EdenMountHandle lookupMount(const MountId& mountId);
  EdenMountHandle lookupMount(const std::unique_ptr<std::string>& mountId);
  EdenMountHandle lookupMount(apache::thrift::field_ref<std::string&> mountId);
  EdenMountHandle lookupMount(
      apache::thrift::field_ref<const std::string&> mountId);
  EdenMountHandle lookupMount(const std::string& mountId);

  void fillDaemonInfo(DaemonInfo& info);

  /**
   * Returns attributes requested by `reqBitmask` for each path in `paths`.
   *
   * The caller must ensure `edenMount` and `paths` stay alive for the duration
   * of the operation.
   */
  ImmediateFuture<std::vector<folly::Try<EntryAttributes>>> getEntryAttributes(
      const EdenMount& edenMount,
      const std::vector<std::string>& paths,
      EntryAttributeFlags reqBitmask,
      AttributesRequestScope reqScope,
      SyncBehavior sync,
      const ObjectFetchContextPtr& fetchContext);
  ImmediateFuture<EntryAttributes> getEntryAttributesForPath(
      const EdenMount& edenMount,
      EntryAttributeFlags reqBitmask,
      AttributesRequestScope reqScope,
      std::string_view path,
      const ObjectFetchContextPtr& fetchContext);

  folly::Synchronized<std::unordered_map<uint64_t, ThriftRequestTraceEvent>>
      outstandingThriftRequests_;

  const std::vector<std::string> originalCommandLine_;
  EdenServer* const server_;

  std::unique_ptr<UsageService> usageService_;

  std::optional<ActivityBuffer<ThriftRequestTraceEvent>>
      thriftRequestActivityBuffer_;

  TraceSubscriptionHandle<ThriftRequestTraceEvent> thriftRequestTraceHandle_;

  std::shared_ptr<TraceBus<ThriftRequestTraceEvent>> thriftRequestTraceBus_;
};
} // namespace facebook::eden
