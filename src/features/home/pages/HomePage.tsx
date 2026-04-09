import { FormEvent, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "react-router-dom";

import packageJson from "../../../../package.json";

import { Panel } from "../../../components/ui/Panel";
import { StatusBadge } from "../../../components/ui/StatusBadge";
import {
  checkCreateWorkspaceTarget,
  createWorkspace,
  listRecentWorkspaces,
  openWorkspace,
  removeRecentWorkspace,
} from "../../../lib/api";
import { pickFolder } from "../../../lib/tauri";
import { RecentWorkspace } from "../../../types/workspace";

type NoticeTone = "success" | "error" | "info";

type NoticeState = {
  tone: NoticeTone;
  title: string;
  detail?: string;
} | null;

function toMessage(error: Error | string) {
  return typeof error === "string" ? error : error.message;
}

function formatTimestamp(value?: string | null) {
  if (!value) {
    return "-";
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return date.toLocaleString();
}

function buildWorkspaceTargetPath(parentPath: string, workspaceName: string) {
  const trimmedParentPath = parentPath.trim().replace(/[\\/]+$/, "");
  const trimmedWorkspaceName = workspaceName.trim();

  if (!trimmedParentPath) {
    return trimmedWorkspaceName;
  }
  if (!trimmedWorkspaceName) {
    return trimmedParentPath;
  }

  return `${trimmedParentPath}\\${trimmedWorkspaceName}`;
}

function validateWorkspaceName(name: string) {
  const trimmedName = name.trim();
  if (!trimmedName) {
    return "Please enter a workspace name before creating it.";
  }
  if (/[<>:"/\\|?*\x00-\x1F]/.test(trimmedName)) {
    return "Workspace name contains characters that are not allowed on Windows.";
  }
  if (/[ .]$/.test(trimmedName)) {
    return "Workspace name cannot end with a space or period on Windows.";
  }

  const reservedNames = new Set([
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
  ]);
  if (reservedNames.has(trimmedName.toUpperCase())) {
    return "Workspace name uses a reserved Windows device name.";
  }

  return null;
}

export function HomePage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [createName, setCreateName] = useState("");
  const [createPath, setCreatePath] = useState("");
  const [openPath, setOpenPath] = useState("");
  const [notice, setNotice] = useState<NoticeState>(null);
  const [pickerTarget, setPickerTarget] = useState<"create" | "open" | null>(null);
  const [pendingRecentId, setPendingRecentId] = useState<string | null>(null);

  const recentQuery = useQuery({
    queryKey: ["recent-workspaces"],
    queryFn: listRecentWorkspaces,
  });
  const workspaces = recentQuery.data ?? [];

  const invalidateRecent = async () => {
    await queryClient.invalidateQueries({ queryKey: ["recent-workspaces"] });
  };

  const createMutation = useMutation({
    mutationFn: createWorkspace,
    onSuccess: async (workspace) => {
      setNotice({
        tone: "success",
        title: `Workspace created: ${workspace.name}`,
        detail: workspace.workspacePath,
      });
      await invalidateRecent();
      navigate(`/workspace/${workspace.id}/sources`);
    },
    onError: (mutationError: Error | string) => {
      setNotice({
        tone: "error",
        title: "Create workspace failed",
        detail: toMessage(mutationError),
      });
    },
  });

  const openMutation = useMutation({
    mutationFn: openWorkspace,
    onSuccess: async (workspace) => {
      setNotice({
        tone: "success",
        title: `Workspace opened: ${workspace.name}`,
        detail: workspace.workspacePath,
      });
      await invalidateRecent();
      navigate(`/workspace/${workspace.id}/browser`);
    },
    onError: (mutationError: Error | string) => {
      setNotice({
        tone: "error",
        title: "Open workspace failed",
        detail: toMessage(mutationError),
      });
    },
  });

  const removeMutation = useMutation({
    mutationFn: removeRecentWorkspace,
    onMutate: (workspaceId) => {
      setPendingRecentId(workspaceId);
    },
    onSuccess: async (_, workspaceId) => {
      setNotice({
        tone: "info",
        title: "Recent workspace removed",
        detail: workspaceId,
      });
      await invalidateRecent();
    },
    onError: (mutationError: Error | string) => {
      setNotice({
        tone: "error",
        title: "Remove recent workspace failed",
        detail: toMessage(mutationError),
      });
    },
    onSettled: () => {
      setPendingRecentId(null);
    },
  });

  const isBusy = createMutation.isPending || openMutation.isPending || removeMutation.isPending;

  const createTargetPath = useMemo(() => buildWorkspaceTargetPath(createPath, createName), [createName, createPath]);

  const recentSummary = useMemo(() => {
    const availableCount = workspaces.filter((workspace) => workspace.available).length;
    if (recentQuery.isLoading) {
      return "Loading recent workspaces...";
    }
    return `${availableCount}/${workspaces.length} paths available`;
  }, [recentQuery.isLoading, workspaces]);

  const busyLabel = useMemo(() => {
    if (createMutation.isPending) {
      return "Creating workspace and initializing SQLite...";
    }
    if (openMutation.isPending) {
      return "Opening workspace and refreshing recent list...";
    }
    if (removeMutation.isPending) {
      return "Updating recent workspace list...";
    }
    if (recentQuery.isFetching) {
      return "Refreshing recent workspaces...";
    }
    return null;
  }, [
    createMutation.isPending,
    openMutation.isPending,
    removeMutation.isPending,
    recentQuery.isFetching,
  ]);

  const handleCreate = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const trimmedName = createName.trim();
    const trimmedParentPath = createPath.trim();
    const nameError = validateWorkspaceName(trimmedName);
    if (nameError) {
      setNotice({
        tone: "error",
        title: "Workspace name is invalid",
        detail: nameError,
      });
      return;
    }

    if (!trimmedParentPath) {
      setNotice({
        tone: "error",
        title: "Workspace parent folder is required",
        detail: "Please choose or enter the parent folder where the workspace should be created.",
      });
      return;
    }

    setNotice(null);

    try {
      const target = await checkCreateWorkspaceTarget({
        name: trimmedName,
        parentPath: trimmedParentPath,
      });

      if (target.status === "existing-workspace") {
        const shouldOpen = window.confirm(
          `The folder already exists as a DataViewer workspace:\n\n${target.targetPath}\n\nOpen the existing workspace instead?`,
        );
        if (shouldOpen) {
          setOpenPath(target.targetPath);
          openMutation.mutate({ workspacePath: target.targetPath });
        }
        return;
      }

      if (target.status === "existing-nonempty") {
        setNotice({
          tone: "error",
          title: "Workspace folder already exists",
          detail: `The target folder already exists and is not empty: ${target.targetPath}. Please choose another workspace name or parent folder.`,
        });
        return;
      }

      const createInput = {
        name: trimmedName,
        parentPath: trimmedParentPath,
        allowExistingTarget: false,
      };

      if (target.status === "existing-empty") {
        const shouldUseExistingEmptyFolder = window.confirm(
          `The target folder already exists and is empty:\n\n${target.targetPath}\n\nInitialize the workspace in this existing folder?`,
        );
        if (!shouldUseExistingEmptyFolder) {
          return;
        }

        createMutation.mutate({
          ...createInput,
          allowExistingTarget: true,
        });
        return;
      }

      createMutation.mutate(createInput);
    } catch (error) {
      setNotice({
        tone: "error",
        title: "Create workspace failed",
        detail: toMessage(error instanceof Error ? error : String(error)),
      });
    }
  };

  const handleOpen = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!openPath.trim()) {
      setNotice({
        tone: "error",
        title: "Workspace folder is required",
        detail: "Enter an existing workspace folder before opening it.",
      });
      return;
    }

    setNotice(null);
    openMutation.mutate({
      workspacePath: openPath.trim(),
    });
  };

  const handleBrowse = async (target: "create" | "open") => {
    setNotice(null);
    setPickerTarget(target);

    try {
      const selectedPath = await pickFolder();
      if (!selectedPath) {
        return;
      }

      if (target === "create") {
        setCreatePath(selectedPath);
      } else {
        setOpenPath(selectedPath);
      }
    } catch (error) {
      setNotice({
        tone: "error",
        title: "Folder picker unavailable",
        detail: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setPickerTarget(null);
    }
  };

  const handleRecentOpen = (workspace: RecentWorkspace) => {
    setNotice(null);
    openMutation.mutate({
      workspacePath: workspace.workspacePath,
    });
  };

  const handleRecentRemove = (workspaceId: string) => {
    setNotice(null);
    removeMutation.mutate(workspaceId);
  };

  return (
    <div className="home-screen">
      <section className="hero-panel">
        <div className="hero-copy">
          <span className="hero-kicker">Desktop Data Curation</span>
          <div className="hero-title-row">
            <h1>DataViewer</h1>
            <span className="version-pill">v{packageJson.version}</span>
          </div>
          <p>
            Organize local COCO, YOLO, and RAW image folders into one visual workspace for
            RF-DETR-ready dataset preparation.
          </p>
          {notice ? (
            <div className={`notice notice-${notice.tone}`} role={notice.tone === "error" ? "alert" : "status"}>
              <strong>{notice.title}</strong>
              {notice.detail ? <span>{notice.detail}</span> : null}
            </div>
          ) : null}
          {busyLabel ? <p className="helper-text">{busyLabel}</p> : null}
        </div>
        <div className="hero-actions">
          <button
            className="button button-primary"
            onClick={() => document.getElementById("create-workspace-name")?.scrollIntoView({ behavior: "smooth" })}
            type="button"
          >
            Create Workspace
          </button>
          <button
            className="button button-secondary"
            onClick={() => document.getElementById("open-workspace-path")?.scrollIntoView({ behavior: "smooth" })}
            type="button"
          >
            Open Existing Workspace
          </button>
        </div>
      </section>

      <div className="export-layout">
        <Panel
          title="Create Workspace"
          subtitle="Choose a parent folder, then DataViewer will create a new workspace folder using the workspace name."
        >
          <form className="form-grid" onSubmit={(event) => void handleCreate(event)}>
            <label className="field">
              <span>Workspace Name</span>
              <input
                id="create-workspace-name"
                disabled={isBusy}
                onChange={(event) => setCreateName(event.target.value)}
                placeholder="Enter workspace name"
                value={createName}
              />
            </label>

            <label className="field">
              <span>Workspace Parent Folder</span>
              <div className="field-inline">
                <input
                  disabled={isBusy}
                  onChange={(event) => setCreatePath(event.target.value)}
                  placeholder="Choose a parent folder for the new workspace"
                  value={createPath}
                />
                <button
                  className="button button-secondary button-sm"
                  disabled={isBusy || pickerTarget === "create"}
                  onClick={() => void handleBrowse("create")}
                  type="button"
                >
                  {pickerTarget === "create" ? "Choosing..." : "Browse Folder"}
                </button>
              </div>
              <span className="helper-text">Workspace will be created at: {createTargetPath || "-"}</span>
            </label>

            <div className="button-row">
              <button className="button button-primary" disabled={isBusy} type="submit">
                {createMutation.isPending ? "Creating..." : "Create Workspace"}
              </button>
            </div>
          </form>
        </Panel>

        <Panel
          title="Open Existing Workspace"
          subtitle="Opens a folder that already contains .dataviewer/workspace.json and workspace.db."
        >
          <form className="form-grid" onSubmit={handleOpen}>
            <label className="field">
              <span>Workspace Folder Path</span>
              <div className="field-inline">
                <input
                  id="open-workspace-path"
                  disabled={isBusy}
                  onChange={(event) => setOpenPath(event.target.value)}
                  placeholder="Choose an existing workspace folder"
                  value={openPath}
                />
                <button
                  className="button button-secondary button-sm"
                  disabled={isBusy || pickerTarget === "open"}
                  onClick={() => void handleBrowse("open")}
                  type="button"
                >
                  {pickerTarget === "open" ? "Choosing..." : "Browse Folder"}
                </button>
              </div>
            </label>

            <div className="button-row">
              <button className="button button-secondary" disabled={isBusy} type="submit">
                {openMutation.isPending ? "Opening..." : "Open Workspace"}
              </button>
            </div>
          </form>
        </Panel>
      </div>

      <Panel title="Recent Workspaces" subtitle={recentSummary}>
        <div className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Location</th>
                <th>Health</th>
                <th>Last Opened</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {workspaces.map((workspace) => {
                const isRemoving = pendingRecentId === workspace.id && removeMutation.isPending;
                const isOpening = openMutation.isPending && openMutation.variables?.workspacePath === workspace.workspacePath;

                return (
                  <tr key={workspace.id}>
                    <td>{workspace.name}</td>
                    <td>{workspace.workspacePath}</td>
                    <td>
                      <StatusBadge status={workspace.available ? workspace.healthStatus : "warning"} />
                    </td>
                    <td>{formatTimestamp(workspace.lastOpenedAt)}</td>
                    <td className="table-actions">
                      {workspace.available ? (
                        <button
                          className="button button-secondary button-sm"
                          disabled={isBusy}
                          onClick={() => handleRecentOpen(workspace)}
                          type="button"
                        >
                          {isOpening ? "Opening..." : "Open"}
                        </button>
                      ) : (
                        <button className="button button-ghost button-sm" disabled type="button">
                          Unavailable
                        </button>
                      )}
                      <button
                        className="button button-ghost button-sm"
                        disabled={isBusy}
                        onClick={() => handleRecentRemove(workspace.id)}
                        type="button"
                      >
                        {isRemoving ? "Removing..." : "Remove"}
                      </button>
                    </td>
                  </tr>
                );
              })}
              {!workspaces.length && !recentQuery.isLoading ? (
                <tr>
                  <td className="empty-row" colSpan={5}>
                    No recent workspaces yet. Create one or open an existing workspace to get started.
                  </td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </Panel>
    </div>
  );
}
