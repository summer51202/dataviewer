type StatusBadgeProps = {
  status: string;
};

const toneMap: Record<string, string> = {
  healthy: "badge-healthy",
  warning: "badge-warning",
  ready: "badge-healthy",
  review: "badge-warning",
  prepared: "badge-info",
  "in progress": "badge-info",
  "ready sync": "badge-warning",
  synced: "badge-healthy",
};

export function StatusBadge({ status }: StatusBadgeProps) {
  const tone = toneMap[status.toLowerCase()] ?? "badge-neutral";
  return <span className={`badge ${tone}`}>{status}</span>;
}
