import { useQuery } from "@tanstack/react-query";
import { useParams } from "react-router-dom";

import { Panel } from "../../../components/ui/Panel";
import { getAnnotationVersions, getExportHistory } from "../../../lib/api";

export function VersionsPage() {
  const { workspaceId = "factory-defect-v1" } = useParams();
  const { data: versions = [] } = useQuery({
    queryKey: ["versions", workspaceId],
    queryFn: () => getAnnotationVersions(workspaceId),
  });
  const { data: exportHistory = [] } = useQuery({
    queryKey: ["export-history", workspaceId],
    queryFn: () => getExportHistory(workspaceId),
  });

  return (
    <div className="page-stack">
      <Panel title="Annotation Versions" subtitle="Each sync from CVAT becomes a new version instead of overwriting previous data.">
        <div className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                <th>Version</th>
                <th>Created At</th>
                <th>Source</th>
                <th>Images</th>
                <th>Boxes</th>
                <th>Notes</th>
              </tr>
            </thead>
            <tbody>
              {versions.map((version) => (
                <tr key={version.id}>
                  <td>{version.label}</td>
                  <td>{version.createdAt}</td>
                  <td>{version.sourceTask}</td>
                  <td>{version.imageCount}</td>
                  <td>{version.boxCount}</td>
                  <td>{version.notes}</td>
                </tr>
              ))}
              {versions.length === 0 ? (
                <tr>
                  <td className="empty-row" colSpan={6}>No annotation versions yet.</td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </Panel>

      <Panel title="Export History" subtitle="Track past exports, output format, destination, and dataset size.">
        <div className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                <th>Created At</th>
                <th>Format</th>
                <th>Status</th>
                <th>Images</th>
                <th>Boxes</th>
                <th>Output Path</th>
              </tr>
            </thead>
            <tbody>
              {exportHistory.map((job) => (
                <tr key={job.id}>
                  <td>{job.createdAt}</td>
                  <td>{job.outputFormat}</td>
                  <td>{job.status}</td>
                  <td>{job.exportedImages}</td>
                  <td>{job.exportedBoxes}</td>
                  <td className="path-cell" title={job.outputPath}>{job.outputPath}</td>
                </tr>
              ))}
              {exportHistory.length === 0 ? (
                <tr>
                  <td className="empty-row" colSpan={6}>No exports recorded yet.</td>
                </tr>
              ) : null}
            </tbody>
          </table>
        </div>
      </Panel>
    </div>
  );
}
