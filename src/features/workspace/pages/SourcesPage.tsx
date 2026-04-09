import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useRef, useState } from "react";
import { useParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import { StatusBadge } from "../../../components/ui/StatusBadge";
import {
  addSourceFolder,
  getScanProgress,
  getSourceFolders,
  removeSourceFolder,
  rescanSourceFolder,
} from "../../../lib/api";
import { describeError, pickFolder } from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/useWorkspaceStore";

type NoticeState =
  | {
      tone: "success" | "error" | "info";
      title: string;
      detail?: string;
    }
  | null;

function formatLastScan(value: string) {
  return value && value !== "-" ? value : "Not scanned yet";
}

export function SourcesPage() {
  const { workspaceId = "factory-defect-v1" } = useParams();
  const queryClient = useQueryClient();
  const resetWorkspaceState = useWorkspaceStore((state) => state.resetWorkspaceState);
  const [notice, setNotice] = useState<NoticeState>(null);
  const [pendingPath, setPendingPath] = useState<string | null>(null);
  const [pendingSourceId, setPendingSourceId] = useState<string | null>(null);
  const [activeScanSeconds, setActiveScanSeconds] = useState(0);
  const [lastProgressChangeAt, setLastProgressChangeAt] = useState<number | null>(null);
  const previousProgressSignatureRef = useRef("");

  const sourceQuery = useQuery({
    queryKey: ["sources", workspaceId],
    queryFn: () => getSourceFolders(workspaceId),
  });
  const sources = sourceQuery.data ?? [];
  const refreshWorkspaceQueries = () => {
    void queryClient.invalidateQueries({ queryKey: ["sources", workspaceId] });
    void queryClient.invalidateQueries({ queryKey: ["workspace-overview", workspaceId] });
    void queryClient.invalidateQueries({ queryKey: ["browser", workspaceId] });
    void queryClient.invalidateQueries({ queryKey: ["import-review", workspaceId] });
    void queryClient.invalidateQueries({ queryKey: ["export-preview", workspaceId] });
    void queryClient.invalidateQueries({ queryKey: ["export-history", workspaceId] });
    void queryClient.invalidateQueries({ queryKey: ["versions", workspaceId] });
    void queryClient.invalidateQueries({ queryKey: ["cvat-tasks", workspaceId] });
  };

  const addSourceMutation = useMutation({
    mutationFn: addSourceFolder,
    onMutate: (input) => {
      setPendingPath(input.sourcePath);
      setNotice(null);
    },
    onSuccess: (_, input) => {
      setNotice({
        tone: "success",
        title: "Source folder added and scanned",
        detail: input.sourcePath,
      });
      refreshWorkspaceQueries();
    },
    onError: (mutationError: unknown) => {
      setNotice({
        tone: "error",
        title: "Add source folder failed",
        detail: describeError(mutationError),
      });
    },
    onSettled: () => {
      setPendingPath(null);
    },
  });

  const rescanMutation = useMutation({
    mutationFn: rescanSourceFolder,
    onMutate: (input) => {
      setPendingSourceId(input.sourceId);
      setNotice(null);
    },
    onSuccess: () => {
      setNotice({
        tone: "info",
        title: "Source folder rescanned",
        detail: "Image, category, browser, and export data have been refreshed.",
      });
      refreshWorkspaceQueries();
    },
    onError: (mutationError: unknown) => {
      setNotice({
        tone: "error",
        title: "Rescan failed",
        detail: describeError(mutationError),
      });
    },
    onSettled: () => {
      setPendingSourceId(null);
    },
  });

  const removeMutation = useMutation({
    mutationFn: removeSourceFolder,
    onMutate: (input) => {
      setPendingSourceId(input.sourceId);
      setNotice(null);
    },
    onSuccess: (updatedSources) => {
      queryClient.setQueryData(["sources", workspaceId], updatedSources);

      if (updatedSources.length === 0) {
        resetWorkspaceState();
        queryClient.setQueryData(["import-review", workspaceId], []);
        queryClient.setQueryData(["browser", workspaceId], {
          sources: [],
          categories: [],
          images: [],
        });
      }

      setNotice({
        tone: "info",
        title: "Source folder removed",
        detail:
          updatedSources.length === 0
            ? "All sources were removed. Browser filters and import review were cleared."
            : "The source was removed from this workspace only.",
      });
      refreshWorkspaceQueries();
    },
    onError: (mutationError: unknown) => {
      setNotice({
        tone: "error",
        title: "Remove source folder failed",
        detail: describeError(mutationError),
      });
    },
    onSettled: () => {
      setPendingSourceId(null);
    },
  });

  const isBusy = addSourceMutation.isPending || rescanMutation.isPending || removeMutation.isPending;

  useEffect(() => {
    if (!isBusy) {
      setActiveScanSeconds(0);
      return;
    }

    const timer = window.setInterval(() => {
      setActiveScanSeconds((current: number) => current + 1);
    }, 1000);

    return () => window.clearInterval(timer);
  }, [isBusy]);

  const progressQuery = useQuery({
    queryKey: ["scan-progress", workspaceId],
    queryFn: () => getScanProgress(workspaceId),
    enabled: isBusy,
    refetchInterval: isBusy ? 250 : false,
  });
  const scanProgress = progressQuery.data ?? [];
  const pendingPathSegments = pendingPath?.split(/[\/]/).filter(Boolean) ?? [];
  const pendingSourceName =
    (pendingPathSegments.length > 0
      ? pendingPathSegments[pendingPathSegments.length - 1]
      : null) ?? "selected folder";
  const progressItems =
    scanProgress.length > 0
      ? scanProgress
      : addSourceMutation.isPending
        ? [
            {
              sourceId: "__pending_add__",
              sourceName: pendingSourceName,
              stage: "Inspecting folders and estimating workload",
              processed: 0,
              total: 0,
            },
          ]
        : [];
  const currentTaskLabel = addSourceMutation.isPending
    ? `Adding source: ${pendingPath ?? "selected folder"}`
    : rescanMutation.isPending
      ? "Rescanning source folder and refreshing import stats..."
      : removeMutation.isPending
        ? "Removing source folder from this workspace..."
        : null;
  const progressSignature = useMemo(
    () =>
      JSON.stringify(
        progressItems.map((progress) => ({
          sourceId: progress.sourceId,
          stage: progress.stage,
          processed: progress.processed,
          total: progress.total,
        })),
      ),
    [progressItems],
  );
  const likelyStalledSeconds =
    lastProgressChangeAt == null
      ? 0
      : Math.max(0, Math.floor((Date.now() - lastProgressChangeAt) / 1000));
  const showLongRunningWarning = isBusy && activeScanSeconds >= 120;
  const showLikelyStalledWarning = isBusy && likelyStalledSeconds >= 45;

  useEffect(() => {
    if (!isBusy) {
      previousProgressSignatureRef.current = "";
      setLastProgressChangeAt(null);
      return;
    }

    if (previousProgressSignatureRef.current !== progressSignature) {
      previousProgressSignatureRef.current = progressSignature;
      setLastProgressChangeAt(Date.now());
    }
  }, [isBusy, progressSignature]);

  const handleAddSourceFolder = async () => {
    setNotice(null);
    const selectedPath = await pickFolder();

    if (!selectedPath) {
      return;
    }

    addSourceMutation.mutate({
      workspaceId,
      sourcePath: selectedPath,
    });
  };

  return (
    <Panel
      title="Sources"
      subtitle="Manage local source folders without modifying original files."
      actions={
        <button
          className="button button-primary"
          disabled={isBusy}
          onClick={() => void handleAddSourceFolder()}
          type="button"
        >
          {addSourceMutation.isPending ? "Adding..." : "Add Source Folder"}
        </button>
      }
    >
      <div className="notice notice-info" role="note">
        <strong>How to choose the folder</strong>
        <span>`COCO`: choose the dataset root. The app looks for `annotations/*.json` or COCO json files in the selected folder, then resolves images from that root.</span>
        <span>`YOLO`: choose a folder that already contains `data.yaml`, or whose top level contains both `images/` and `labels/`.</span>
        <span>If your YOLO data is split as `train/images` and `train/labels` without a top-level `data.yaml`, choose the split folder itself, not the parent folder.</span>
        <span>`RAW images`: choose the folder that directly contains the image files you want to browse.</span>
        <span>Choosing a deeper subfolder, such as COCO `annotations/`, can cause image paths or annotation structure to be missed.</span>
      </div>

      {notice ? (
        <div className={`notice notice-${notice.tone}`} role={notice.tone === "error" ? "alert" : "status"}>
          <strong>{notice.title}</strong>
          {notice.detail ? <span>{notice.detail}</span> : null}
        </div>
      ) : null}
      {currentTaskLabel ? (
        <div className="task-status" role="status">
          <strong>Working</strong>
          <span>{currentTaskLabel}</span>
        </div>
      ) : null}
      {showLongRunningWarning ? (
        <div className="status-banner status-banner-warning status-banner-strong" role="status">
          <strong>Scan is taking longer than expected.</strong>
          <span>
            This scan has been running for more than 2 minutes. Large folders can take a while, especially when many nested images or annotations are being inspected.
          </span>
          <span>
            If the numbers below are still changing, the scan is still working. If they stop changing for a while, it may be stuck.
          </span>
        </div>
      ) : null}
      {showLikelyStalledWarning ? (
        <div className="status-banner status-banner-error status-banner-strong" role="alert">
          <strong>Scan may be stalled.</strong>
          <span>Progress has not changed for about {likelyStalledSeconds} seconds.</span>
          <span>
            The current scan flow does not yet support safe auto-cancel. If this keeps happening on the same folder, try a smaller folder split or restart the action after checking the dataset structure.
          </span>
        </div>
      ) : null}
      {progressItems.length > 0 ? (
        <div className="scan-progress-list">
          {progressItems.map((progress) => {
            const isIndeterminate = progress.total === 0;
            const percent = isIndeterminate
              ? 100
              : Math.min(100, Math.round((progress.processed / progress.total) * 100));
            const progressLabel = isIndeterminate
              ? activeScanSeconds > 0
                ? `Estimating total... ${activeScanSeconds}s`
                : "Estimating total..."
              : `${progress.processed} / ${progress.total}`;
            const stageLabel = isIndeterminate
              ? `${progress.stage}. This can take a bit on large folders.`
              : progress.stage;

            return (
              <div className="scan-progress-card" key={progress.sourceId}>
                <div className="scan-progress-head">
                  <strong>{progress.sourceName}</strong>
                  <span>{progressLabel}</span>
                </div>
                <div className="scan-progress-stage">{stageLabel}</div>
                <div className="scan-progress-bar">
                  <div
                    className={`scan-progress-bar-fill${isIndeterminate ? " scan-progress-bar-fill-indeterminate" : ""}`}
                    style={{ width: `${percent}%` }}
                  />
                </div>
              </div>
            );
          })}
        </div>
      ) : null}

      <div className="table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Source Name</th>
              <th>Source Path</th>
              <th>Type</th>
              <th>Status</th>
              <th>Images</th>
              <th>Corrupted</th>
              <th>Categories</th>
              <th>Last Scan</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {sources.map((source) => {
              const isPendingRow =
                (pendingPath?.endsWith(source.name) && addSourceMutation.isPending) ||
                (pendingSourceId === source.id && (rescanMutation.isPending || removeMutation.isPending));

              return (
                <tr key={source.id}>
                  <td>{source.name}</td>
                  <td>
                    <div className="path-cell" title={source.path}>{source.path}</div>
                  </td>
                  <td>{source.type}</td>
                  <td>
                    <StatusBadge status={source.status} />
                    {source.status === "warning" && source.corruptedImageCount > 0 ? (
                      <div className="helper-text">
                        Warning: {source.corruptedImageCount} corrupted image{source.corruptedImageCount === 1 ? "" : "s"} detected. They will be skipped in Browser and Export.
                      </div>
                    ) : null}
                  </td>
                  <td>{source.imageCount}</td>
                  <td>
                    {source.corruptedImageCount > 0 ? (
                      <div className="path-cell" title={source.corruptedImagePaths.join("\n")}>
                        {source.corruptedImageCount}
                      </div>
                    ) : (
                      0
                    )}
                  </td>
                  <td>{source.categoryCount}</td>
                  <td>{formatLastScan(source.lastScanAt)}</td>
                  <td className="table-actions">
                    <button
                      className="button button-secondary button-sm"
                      disabled={isBusy}
                      onClick={() => rescanMutation.mutate({ workspaceId, sourceId: source.id })}
                      type="button"
                    >
                      {rescanMutation.isPending && pendingSourceId === source.id ? "Scanning..." : isPendingRow ? "Queued..." : "Rescan"}
                    </button>
                    <button
                      className="button button-ghost button-sm"
                      disabled={isBusy}
                      onClick={() => removeMutation.mutate({ workspaceId, sourceId: source.id })}
                      type="button"
                    >
                      {removeMutation.isPending && pendingSourceId === source.id ? "Removing..." : "Remove"}
                    </button>
                  </td>
                </tr>
              );
            })}
            {!sources.length && !sourceQuery.isLoading ? (
              <tr>
                <td className="empty-row" colSpan={9}>
                  No source folders yet. Use "Add Source Folder" to start building this workspace.
                </td>
              </tr>
            ) : null}
            {sourceQuery.isLoading ? (
              <tr>
                <td className="empty-row" colSpan={9}>
                  Loading source folders...
                </td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}
