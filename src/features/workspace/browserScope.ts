import { AnnotationCountFilter, AnnotationFilter, BrowserPayload, ImageCard } from "../../types/workspace";

export type BrowserFilterState = {
  search: string;
  annotationFilter: AnnotationFilter;
  selectedSourceIds: string[];
  selectedCategoryIds: string[];
  annotationCountFilter: AnnotationCountFilter;
  minBoxAreaRatioPercent: string;
};

export type ExportScopeMode = "filtered" | "selected";

function matchesAnnotationCountFilter(image: ImageCard, filter: AnnotationCountFilter) {
  if (filter === "") {
    return true;
  }

  if (filter === "5+") {
    return image.annotationCount >= 5;
  }

  return image.annotationCount === Number.parseInt(filter, 10);
}

export function filterBrowserImages(images: ImageCard[], filters: BrowserFilterState) {
  const trimmedSearch = filters.search.trim().toLowerCase();
  const selectedSourceIdSet = new Set(filters.selectedSourceIds);
  const selectedCategoryIdSet = new Set(filters.selectedCategoryIds);
  const minBoxAreaRatio =
    filters.minBoxAreaRatioPercent.trim() === ""
      ? null
      : Math.min(100, Math.max(0, Number.parseFloat(filters.minBoxAreaRatioPercent))) / 100;

  return images.filter((image) => {
    const sourceMatch =
      selectedSourceIdSet.size === 0 || selectedSourceIdSet.has(image.sourceId);
    const categoryMatch =
      selectedCategoryIdSet.size === 0 ||
      image.categoryIds.some((categoryId) => selectedCategoryIdSet.has(categoryId));
    const annotationMatch =
      filters.annotationFilter === "all" || image.annotationStatus === filters.annotationFilter;
    const searchMatch =
      trimmedSearch.length === 0 || image.filename.toLowerCase().includes(trimmedSearch);
    const annotationCountMatch = matchesAnnotationCountFilter(image, filters.annotationCountFilter);
    const boxRatioMatch =
      minBoxAreaRatio == null ||
      (image.maxBoxAreaRatio != null && image.maxBoxAreaRatio >= minBoxAreaRatio);

    return (
      sourceMatch &&
      categoryMatch &&
      annotationMatch &&
      searchMatch &&
      annotationCountMatch &&
      boxRatioMatch
    );
  });
}

export function resolveExportScopeImages(
  payload: BrowserPayload | undefined,
  filters: BrowserFilterState,
  selectedImageIds: string[],
  scopeMode: ExportScopeMode,
) {
  const images = payload?.images ?? [];
  const filteredImages = filterBrowserImages(images, filters);

  if (scopeMode === "selected") {
    const selectedImageIdSet = new Set(selectedImageIds);
    return images.filter((image) => selectedImageIdSet.has(image.id));
  }

  return filteredImages;
}
