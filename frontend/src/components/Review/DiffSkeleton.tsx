export function DiffSkeleton() {
  return (
    <div className="flex h-full flex-1 flex-col overflow-hidden">
      {/* Mini-toolbar placeholder */}
      <div className="border-border bg-bg-primary h-10 border-b" />

      {/* Editor-like area */}
      <div className="flex-1 animate-pulse overflow-hidden bg-bg-primary p-6 font-mono">
        <div className="space-y-4">
          <div className="flex gap-4">
            <div className="bg-bg-tertiary h-4 w-8 rounded opacity-50" />
            <div className="bg-bg-tertiary h-4 w-3/4 rounded" />
          </div>
          <div className="flex gap-4">
            <div className="bg-bg-tertiary h-4 w-8 rounded opacity-50" />
            <div className="bg-bg-tertiary h-4 w-5/6 rounded" />
          </div>
          <div className="bg-bg-tertiary/10 flex gap-4 py-1">
            <div className="bg-bg-tertiary h-4 w-8 rounded opacity-50" />
            <div className="bg-bg-tertiary h-4 w-4/6 rounded" />
          </div>
          <div className="flex gap-4">
            <div className="bg-bg-tertiary h-4 w-8 rounded opacity-50" />
            <div className="bg-bg-tertiary h-4 w-2/3 rounded" />
          </div>
          <div className="flex gap-4">
            <div className="bg-bg-tertiary h-4 w-8 rounded opacity-50" />
            <div className="bg-bg-tertiary h-4 w-1/2 rounded" />
          </div>
          <div className="bg-bg-tertiary/5 mt-8 h-px w-full" />
          <div className="flex gap-4">
            <div className="bg-bg-tertiary h-4 w-8 rounded opacity-50" />
            <div className="bg-bg-tertiary h-4 w-3/4 rounded" />
          </div>
        </div>
      </div>
    </div>
  );
}
