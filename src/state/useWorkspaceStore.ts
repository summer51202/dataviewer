import { create } from "zustand";

import { AnnotationCountFilter, AnnotationFilter } from "../types/workspace";

type WorkspaceState = {
  selectedSourceIds: string[];
  selectedCategoryIds: string[];
  annotationFilter: AnnotationFilter;
  search: string;
  annotationCountFilter: AnnotationCountFilter;
  minBoxAreaRatioPercent: string;
  selectedImageIds: string[];
  toggleSource: (id: string) => void;
  toggleCategory: (id: string) => void;
  setAnnotationFilter: (value: AnnotationFilter) => void;
  setSearch: (value: string) => void;
  setAnnotationCountFilter: (value: AnnotationCountFilter) => void;
  setMinBoxAreaRatioPercent: (value: string) => void;
  toggleImageSelection: (imageId: string) => void;
  setSelectedImageIds: (imageIds: string[]) => void;
  resetWorkspaceState: () => void;
};

function toggleEntry(items: string[], value: string) {
  const nextItems = new Set(items);

  if (nextItems.has(value)) {
    nextItems.delete(value);
  } else {
    nextItems.add(value);
  }

  return Array.from(nextItems);
}

export const useWorkspaceStore = create<WorkspaceState>((set) => ({
  selectedSourceIds: [],
  selectedCategoryIds: [],
  annotationFilter: "all",
  search: "",
  annotationCountFilter: "",
  minBoxAreaRatioPercent: "",
  selectedImageIds: [],
  toggleSource: (id) =>
    set((state) => ({ selectedSourceIds: toggleEntry(state.selectedSourceIds, id) })),
  toggleCategory: (id) =>
    set((state) => ({ selectedCategoryIds: toggleEntry(state.selectedCategoryIds, id) })),
  setAnnotationFilter: (value) => set({ annotationFilter: value }),
  setSearch: (value) => set({ search: value }),
  setAnnotationCountFilter: (value) => set({ annotationCountFilter: value }),
  setMinBoxAreaRatioPercent: (value) => set({ minBoxAreaRatioPercent: value }),
  toggleImageSelection: (imageId) =>
    set((state) => ({ selectedImageIds: toggleEntry(state.selectedImageIds, imageId) })),
  setSelectedImageIds: (imageIds) => set({ selectedImageIds: imageIds }),
  resetWorkspaceState: () =>
    set({
      selectedSourceIds: [],
      selectedCategoryIds: [],
      annotationFilter: "all",
      search: "",
      annotationCountFilter: "",
      minBoxAreaRatioPercent: "",
      selectedImageIds: [],
    }),
}));
