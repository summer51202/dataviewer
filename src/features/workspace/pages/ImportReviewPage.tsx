import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import { getImportReview, saveImportReview } from "../../../lib/api";
import { describeError } from "../../../lib/tauri";
import { ImportReviewRow } from "../../../types/workspace";

export function ImportReviewPage() {
  const { workspaceId = "factory-defect-v1" } = useParams();
  const queryClient = useQueryClient();
  const { data: rows = [] } = useQuery({
    queryKey: ["import-review", workspaceId],
    queryFn: () => getImportReview(workspaceId),
  });
  const [draftRows, setDraftRows] = useState<ImportReviewRow[]>([]);

  useEffect(() => {
    setDraftRows(rows);
  }, [rows]);

  const saveMutation = useMutation({
    mutationFn: (nextRows: ImportReviewRow[]) =>
      saveImportReview({ workspaceId, rows: nextRows }),
    onSuccess: (savedRows) => {
      queryClient.setQueryData(["import-review", workspaceId], savedRows);
      setDraftRows(savedRows);
      void queryClient.invalidateQueries({ queryKey: ["browser", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["workspace-overview", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["export-preview", workspaceId] });
      void queryClient.invalidateQueries({ queryKey: ["image-detail", workspaceId] });
    },
  });

  const updateRow = (sourceCategoryId: string, patch: Partial<ImportReviewRow>) => {
    if (saveMutation.isError || saveMutation.isSuccess) {
      saveMutation.reset();
    }

    setDraftRows((current) =>
      current.map((row) =>
        row.sourceCategoryId === sourceCategoryId ? { ...row, ...patch } : row,
      ),
    );
  };

  const handleSave = () => {
    if (saveMutation.isError || saveMutation.isSuccess) {
      saveMutation.reset();
    }

    saveMutation.mutate(draftRows);
  };

  return (
    <Panel
      title="Import Review"
      subtitle="Confirm how incoming source categories merge into workspace-level unified categories."
      actions={
        <button
          className="button button-primary"
          disabled={saveMutation.isPending || draftRows.length === 0}
          onClick={handleSave}
          type="button"
        >
          {saveMutation.isPending ? "Saving..." : "Save Mapping"}
        </button>
      }
    >
      {saveMutation.isError ? (
        <div className="status-banner status-banner-error">
          <strong>Failed to save import review mapping.</strong>
          <span>{describeError(saveMutation.error)}</span>
          <span>Edit the rows below and save again.</span>
        </div>
      ) : null}
      {saveMutation.isSuccess ? (
        <div className="status-banner status-banner-success">
          Import review mapping saved.
        </div>
      ) : null}
      <div className="table-shell">
        <table className="data-table">
          <thead>
            <tr>
              <th>Source Category</th>
              <th>Source Path</th>
              <th>Count</th>
              <th>Suggested</th>
              <th>Target Unified Category</th>
              <th>Final Action</th>
            </tr>
          </thead>
          <tbody>
            {draftRows.map((row) => (
              <tr key={row.sourceCategoryId}>
                <td>{row.sourceCategory}</td>
                <td>
                  <div className="path-cell" title={row.sourcePath}>{row.sourcePath}</div>
                </td>
                <td>{`${row.count}/${row.sourceTotalImageCount}`}</td>
                <td>{row.suggestedAction}</td>
                <td>
                  <input
                    className="search-input"
                    disabled={saveMutation.isPending || row.finalAction === "Ignore"}
                    onChange={(event) =>
                      updateRow(row.sourceCategoryId, {
                        targetUnifiedCategory: event.target.value,
                      })
                    }
                    value={row.targetUnifiedCategory ?? ""}
                  />
                </td>
                <td>
                  <select
                    className="search-input"
                    disabled={saveMutation.isPending}
                    onChange={(event) =>
                      updateRow(row.sourceCategoryId, {
                        finalAction: event.target.value as ImportReviewRow["finalAction"],
                      })
                    }
                    value={row.finalAction}
                  >
                    <option value="Merge">Merge</option>
                    <option value="Create New">Create New</option>
                    <option value="Ignore">Ignore</option>
                  </select>
                </td>
              </tr>
            ))}
            {draftRows.length === 0 ? (
              <tr>
                <td className="empty-row" colSpan={6}>
                  No source category mappings need review right now. Add a source folder to populate this page.
                </td>
              </tr>
            ) : null}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}
