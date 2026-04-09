import { useQuery } from "@tanstack/react-query";
import { Link, NavLink, Outlet, useParams } from "react-router-dom";

import { PageHeader } from "../../components/ui/PageHeader";
import { Panel } from "../../components/ui/Panel";
import { StatusBadge } from "../../components/ui/StatusBadge";
import { getWorkspaceOverview } from "../../lib/api";
import { cx } from "../../lib/cx";

const navItems = [
  { to: "browser", label: "Browser" },
  { to: "sources", label: "Sources" },
  { to: "import-review", label: "Import Review" },
  { to: "cvat", label: "CVAT Tasks" },
  { to: "versions", label: "Versions" },
  { to: "export", label: "Export" },
];

export function WorkspaceLayout() {
  const { workspaceId = "factory-defect-v1" } = useParams();
  const { data: overview } = useQuery({
    queryKey: ["workspace-overview", workspaceId],
    queryFn: () => getWorkspaceOverview(workspaceId),
  });

  if (!overview) {
    return null;
  }

  return (
    <div className="workspace-shell">
      <aside className="workspace-sidebar">
        <div className="brand-block">
          <div className="brand-kicker">DataViewer</div>
          <div className="brand-title">{overview.name}</div>
          <div className="brand-subtle brand-path" title={overview.workspacePath}>
            {overview.workspacePath}
          </div>
          <Link className="sidebar-home-link" to="/">
            Return To Workspace Home
          </Link>
        </div>

        <nav className="nav-list">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) => cx("nav-link", isActive && "nav-link-active")}
            >
              {item.label}
            </NavLink>
          ))}
        </nav>

        <Panel title="Sources Summary" subtitle={`${overview.sources.length} sources loaded`}>
          <ul className="compact-list">
            {overview.sources.map((source) => (
              <li key={source.id}>
                <span>{source.name}</span>
                <StatusBadge status={source.status} />
              </li>
            ))}
          </ul>
        </Panel>

        <Panel title="Categories Summary" subtitle={`${overview.categories.length} unified classes`}>
          <ul className="compact-list">
            {overview.categories.map((category) => (
              <li key={category.id}>
                <span>{category.name}</span>
                <strong>{category.imageCount}</strong>
              </li>
            ))}
          </ul>
        </Panel>
      </aside>

      <main className="workspace-main">
        <PageHeader
          title={overview.name}
          description="Multi-folder object detection workspace for COCO, YOLO, and RAW image datasets."
          actions={
            <>
              <StatusBadge status={overview.healthStatus} />
              <button className="button button-secondary" type="button">
                Rescan
              </button>
            </>
          }
        />
        <Outlet />
      </main>
    </div>
  );
}
