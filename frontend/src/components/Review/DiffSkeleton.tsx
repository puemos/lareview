export function DiffSkeleton() {
  return (
    <div className="flex flex-1 flex-col">
      <div className="border-border bg-bg-primary h-10 border-b" />
      <div className="flex-1 animate-pulse space-y-3 p-4">
        <div className="bg-bg-tertiary h-4 w-full rounded" />
        <div className="bg-bg-tertiary h-4 w-5/6 rounded" />
        <div className="bg-bg-tertiary h-4 w-4/6 rounded" />
      </div>
    </div>
  );
}
