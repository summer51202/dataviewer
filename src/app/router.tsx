import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";

import { WorkspaceLayout } from "./layout/WorkspaceLayout";
import { HomePage } from "../features/home/pages/HomePage";
import { BrowserPage } from "../features/workspace/pages/BrowserPage";
import { SourcesPage } from "../features/workspace/pages/SourcesPage";
import { ImportReviewPage } from "../features/workspace/pages/ImportReviewPage";
import { CvatTasksPage } from "../features/workspace/pages/CvatTasksPage";
import { VersionsPage } from "../features/workspace/pages/VersionsPage";
import { ExportPage } from "../features/workspace/pages/ExportPage";
import { ImageDetailPage } from "../features/workspace/pages/ImageDetailPage";

export function AppRouter() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/workspace/:workspaceId" element={<WorkspaceLayout />}>
          <Route index element={<Navigate replace to="browser" />} />
          <Route path="browser" element={<BrowserPage />} />
          <Route path="sources" element={<SourcesPage />} />
          <Route path="import-review" element={<ImportReviewPage />} />
          <Route path="cvat" element={<CvatTasksPage />} />
          <Route path="versions" element={<VersionsPage />} />
          <Route path="export" element={<ExportPage />} />
          <Route path="image/:imageId" element={<ImageDetailPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
