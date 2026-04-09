import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import { StatusBadge } from "../../../components/ui/StatusBadge";
import {
  createCvatTask,
  getBrowserPayload,
  getCvatSettings,
  getCvatTasks,
  openCvat,
  openExportFolder,
  saveCvatSettings,
  syncCvatTask,
  testCvatSettings,
} from "../../../lib/api";
import { describeError } from "../../../lib/tauri";
import { useWorkspaceStore } from "../../../state/useWorkspaceStore";

function formatSyncTime(value: string | null | undefined) {
  return value && value !== "-" ? value : "-";
}

export function CvatTasksPage() {
  const { workspaceId = "factory-defect-v1" } = useParams();
  const queryClient = useQueryClient();
  const selectedImageIds = useWorkspaceStore((state) => state.selectedImageIds);
  const [taskName, setTaskName] = useState("");
  const [feedback, setFeedback] = useState<string | null>(null);
  const [feedbackTone, setFeedbackTone] = useState<"success" | "error" | null>(null);
  const [baseUrl, setBaseUrl] = useState("http://localhost:8080");
  const [accessToken, setAccessToken] = useState("");

  const { data: tasks = [] } = useQuery({
    queryKey: ["cvat-tasks", workspaceId],
    queryFn: () => getCvatTasks(workspaceId),
  });
  const { data: browserPayload } = useQuery({
    queryKey: ["browser", workspaceId],
    queryFn: () => getBrowserPayload(workspaceId),
  });
  const { data: cvatSettings } = useQuery({
    queryKey: ["cvat-settings", workspaceId],
    queryFn: () => getCvatSettings(workspaceId),
    retry: false,
  });

  useEffect(() => {
    if (!cvatSettings) {
      return;
    }
    setBaseUrl(cvatSettings.baseUrl || "http://localhost:8080");
    setAccessToken(cvatSettings.accessToken || "");
  }, [cvatSettings]);

  const selectedImageIdSet = useMemo(() => new Set(selectedImageIds), [selectedImageIds]);
  const imageCategoryMap = useMemo(
    () =>
      new Map(
        (browserPayload?.images ?? []).map((image) => [image.id, image.categories] as const),
      ),
    [browserPayload],
  );

  const selectionSummary = useMemo(() => {
    const categories = Array.from(
      selectedImageIds.reduce((accumulator, imageId) => {
        const imageCategories = imageCategoryMap.get(imageId);

        if (!imageCategories) {
          return accumulator;
        }

        imageCategories.forEach((category) => accumulator.add(category));
        return accumulator;
      }, new Set<string>()),
    ).sort();

    return {
      imageCount: selectedImageIdSet.size,
      categories,
    };
  }, [imageCategoryMap, selectedImageIdSet, selectedImageIds]);

  const createMutation = useMutation({
    mutationFn: () =>
      createCvatTask({
        workspaceId,
        imageIds: selectedImageIds,
        taskName: taskName.trim() || undefined,
      }),
    onMutate: () => {
      setFeedback(null);
      setFeedbackTone(null);
    },
    onSuccess: (updatedTasks) => {
      queryClient.setQueryData(["cvat-tasks", workspaceId], updatedTasks);
      setTaskName("");
      const latestTask = updatedTasks[0];
      setFeedback(
        latestTask?.remoteTaskId
          ? `Remote CVAT task created and uploaded: ${latestTask.name}`
          : latestTask?.tempFolder
            ? `Local CVAT staging task created: ${latestTask.tempFolder}`
            : "CVAT task created.",
      );
      setFeedbackTone("success");
    },
    onError: (error: unknown) => {
      setFeedback(describeError(error));
      setFeedbackTone("error");
    },
  });

  const saveSettingsMutation = useMutation({
    mutationFn: () =>
      saveCvatSettings(workspaceId, {
        baseUrl: baseUrl.trim(),
        accessToken: accessToken.trim(),
      }),
    onMutate: () => {
      setFeedback(null);
      setFeedbackTone(null);
    },
    onSuccess: (settings) => {
      queryClient.setQueryData(["cvat-settings", workspaceId], settings);
      setBaseUrl(settings.baseUrl);
      setAccessToken(settings.accessToken);
      setFeedback("CVAT connection settings saved.");
      setFeedbackTone("success");
    },
    onError: (error: unknown) => {
      setFeedback(describeError(error));
      setFeedbackTone("error");
    },
  });

  const testSettingsMutation = useMutation({
    mutationFn: () => testCvatSettings(workspaceId),
    onMutate: () => {
      setFeedback(null);
      setFeedbackTone(null);
    },
    onSuccess: () => {
      setFeedback("CVAT connection test succeeded.");
      setFeedbackTone("success");
    },
    onError: (error: unknown) => {
      setFeedback(describeError(error));
      setFeedbackTone("error");
    },
  });

  const openCvatMutation = useMutation({
    mutationFn: (taskId?: string) => openCvat({ workspaceId, taskId }),
    onMutate: () => {
      setFeedback(null);
      setFeedbackTone(null);
    },
    onError: (error: unknown) => {
      setFeedback(describeError(error));
      setFeedbackTone("error");
    },
  });

  const syncMutation = useMutation({
    mutationFn: (taskId: string) => syncCvatTask({ workspaceId, taskId }),
    onMutate: () => {
      setFeedback(null);
      setFeedbackTone(null);
    },
    onSuccess: (updatedTasks) => {
      queryClient.setQueryData(["cvat-tasks", workspaceId], updatedTasks);
      void queryClient.invalidateQueries({ queryKey: ["versions", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["browser", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["workspace-overview", workspaceId] });
      setFeedback("CVAT annotations synced back into a new annotation version.");
      setFeedbackTone("success");
    },
    onError: (error: unknown) => {
      setFeedback(describeError(error));
      setFeedbackTone("error");
    },
  });

  const openFolderMutation = useMutation({
    mutationFn: (path: string) => openExportFolder(path),
    onMutate: () => {
      setFeedback(null);
      setFeedbackTone(null);
    },
    onError: (error: unknown) => {
      setFeedback(describeError(error));
      setFeedbackTone("error");
    },
  });

  return (
    <div className="stack">
      <Panel title="CVAT Connection" subtitle="Save local CVAT API settings to enable remote task creation and direct task opening.">
        {feedback && feedbackTone ? (
          <div className={`status-banner status-banner-${feedbackTone}`}>
            <span>{feedback}</span>
          </div>
        ) : null}
        <div className="form-grid">
          <label className="field">
            <span>Base URL</span>
            <input
              onChange={(event) => setBaseUrl(event.target.value)}
              placeholder="http://localhost:8080"
              type="text"
              value={baseUrl}
            />
            <span className="field-help">Example: http://localhost:8080</span>
          </label>
          <label className="field">
            <span>Access Token</span>
            <input
              onChange={(event) => setAccessToken(event.target.value)}
              placeholder="Paste a CVAT personal access token"
              type="password"
              value={accessToken}
            />
            <span className="field-help">Used for create task, upload images, and open remote tasks.</span>
          </label>
        </div>
        <div className="button-row">
          <button
            className="button button-secondary"
            disabled={testSettingsMutation.isPending}
            onClick={() => testSettingsMutation.mutate()}
            type="button"
          >
            {testSettingsMutation.isPending ? "Testing..." : "Test Connection"}
          </button>
          <button
            className="button button-primary"
            disabled={saveSettingsMutation.isPending}
            onClick={() => saveSettingsMutation.mutate()}
            type="button"
          >
            {saveSettingsMutation.isPending ? "Saving..." : "Save Settings"}
          </button>
          <button
            className="button button-secondary"
            disabled={openCvatMutation.isPending}
            onClick={() => openCvatMutation.mutate(undefined)}
            type="button"
          >
            {openCvatMutation.isPending ? "Opening..." : "Open CVAT Home"}
          </button>
        </div>
      </Panel>

      <Panel title="CVAT Tasks" subtitle="Create local staging tasks, or if CVAT settings are configured, push the staged images to CVAT automatically.">
        <div className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                <th>Task Name</th>
                <th>Image Count</th>
                <th>Status</th>
                <th>CVAT Mode</th>
                <th>Last Sync</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {tasks.map((task) => {
                const canSync = task.status.toLowerCase() === "ready sync" || task.status.toLowerCase() === "synced";

                return (
                  <tr key={task.id}>
                    <td>
                      <div>{task.name}</div>
                      {task.remoteUrl ? <div className="path-cell" title={task.remoteUrl}>{task.remoteUrl}</div> : null}
                      {!task.remoteUrl && task.tempFolder ? <div className="path-cell" title={task.tempFolder}>{task.tempFolder}</div> : null}
                    </td>
                    <td>{task.imageCount}</td>
                    <td>
                      <StatusBadge status={task.status} />
                    </td>
                    <td>{task.projectName}</td>
                    <td>{formatSyncTime(task.lastSyncAt)}</td>
                    <td className="table-actions">
                      <button
                        className="button button-secondary button-sm"
                        disabled={openCvatMutation.isPending}
                        onClick={() => openCvatMutation.mutate(task.id)}
                        type="button"
                      >
                        {openCvatMutation.isPending ? "Opening..." : task.remoteTaskId ? "Open Remote" : "Open CVAT"}
                      </button>
                      <button
                        className="button button-secondary button-sm"
                        disabled={!task.tempFolder || openFolderMutation.isPending}
                        onClick={() => task.tempFolder && openFolderMutation.mutate(task.tempFolder)}
                        type="button"
                      >
                        {openFolderMutation.isPending ? "Opening..." : "Open Staging"}
                      </button>
                      <button
                        className="button button-primary button-sm"
                        disabled={!canSync || syncMutation.isPending}
                        onClick={() => syncMutation.mutate(task.id)}
                        type="button"
                      >
                        {syncMutation.isPending ? "Syncing..." : "Sync Back"}
                      </button>
                    </td>
                  </tr>
                );
              })}
              {tasks.length === 0 ? (
                <tr>
                  <td className="empty-row" colSpan={6}>
                    No CVAT tasks yet. Create one from the current browser selection below.
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </Panel>

      <Panel title="Create New CVAT Task" subtitle="Selected unannotated images will always be staged locally. If CVAT settings are saved, the app will also create a remote task and upload the images automatically.">
        <div className="form-grid">
          <label className="field">
            <span>Task Name</span>
            <input
              onChange={(event) => setTaskName(event.target.value)}
              placeholder="Optional. Leave empty to use a timestamped name."
              type="text"
              value={taskName}
            />
            <span className="field-help">Manual sync-back still reads COCO json from the staging folder for this phase.</span>
          </label>
        </div>
        <ul className="compact-list">
          <li>
            <span>Current selection</span>
            <strong>{selectionSummary.imageCount} images</strong>
          </li>
          <li>
            <span>Detected labels</span>
            <strong>
              {selectionSummary.categories.length > 0
                ? selectionSummary.categories.join(", ")
                : "CVAT will use existing workspace unified categories"}
            </strong>
          </li>
          <li>
            <span>Expected sync file</span>
            <strong>annotations/instances_default.json</strong>
          </li>
        </ul>
        <div className="button-row">
          <button
            className="button button-primary"
            disabled={selectionSummary.imageCount === 0 || createMutation.isPending}
            onClick={() => createMutation.mutate()}
            type="button"
          >
            {createMutation.isPending ? "Preparing..." : "Create CVAT Task from Selection"}
          </button>
        </div>
      </Panel>
    </div>
  );
}
