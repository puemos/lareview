import {
  Circle,
  CircleDashed,
  Spinner,
  CheckCircle,
  XCircle,
  CaretCircleDown,
  CaretCircleUp,
  CaretCircleDoubleUp,
  Asterisk,
  Eyes,
  Folder,
  Gear,
  FileText,
  ChartBar,
  Files,
  ChatCircle,
  ChatTeardrop,
  Play,
  Stop,
  TrashSimple,
  Trash,
  Export,
  PaperPlaneRight,
  X,
  ArrowClockwise,
  ArrowSquareOut,
  ArrowsOutSimple,
  ArrowsInSimple,
  ArrowSquareIn,
  FloppyDisk,
  Copy,
  MagnifyingGlass,
  ListChecks,
  GithubLogo,
  BoundingBox,
  Warning,
  Plus,
  Minus,
  Square,
  CheckSquare,
  DotOutline,
  ArrowRight,
  CaretDown,
  HandPalm,
  Lightbulb,
  Microscope,
} from '@phosphor-icons/react';

/**
 * Centralized icon registry for LaReview.
 *
 * All UI components should use these constants instead of directly
 * importing from `@phosphor-icons/react` to ensure visual consistency.
 */
export const ICONS = {
  // --- Task & Feedback Status ---
  STATUS_TODO: Circle,
  STATUS_IN_PROGRESS: CircleDashed,
  STATUS_DONE: CheckCircle,
  STATUS_IGNORED: XCircle,

  // --- Risk Levels ---
  RISK_LOW: CaretCircleDown,
  RISK_MEDIUM: CaretCircleUp,
  RISK_HIGH: CaretCircleDoubleUp,

  // --- Navigation & Views ---
  VIEW_GENERATE: Asterisk,
  VIEW_REVIEW: Eyes,
  VIEW_REPOS: Folder,
  VIEW_SETTINGS: Gear,

  TAB_DESCRIPTION: FileText,
  TAB_DIAGRAM: ChartBar,
  TAB_CHANGES: Files,
  TAB_FEEDBACK: ChatCircle,

  // --- Common Actions ---
  ACTION_RUN: Play,
  ACTION_STOP: Stop,
  ACTION_DELETE: TrashSimple,
  ACTION_TRASH: Trash,
  ACTION_EXPORT: Export,
  ACTION_REPLY: PaperPlaneRight,
  ACTION_CLOSE: X,
  ACTION_CLEAR: Trash,
  ACTION_REFRESH: ArrowClockwise,
  ACTION_OPEN_WINDOW: ArrowSquareOut,
  ACTION_EXPAND: ArrowsOutSimple,
  ACTION_COLLAPSE: ArrowsInSimple,
  ACTION_BACK: ArrowSquareIn,
  ACTION_COPY: Copy,
  ACTION_SAVE: FloppyDisk,
  ACTION_SEARCH: MagnifyingGlass,
  ACTION_LOADING: Spinner,

  // --- Symbols ---
  ICON_PLAN: ListChecks,
  ICON_FEEDBACK: ChatTeardrop,
  ICON_GITHUB: GithubLogo,
  ICON_EMPTY: BoundingBox,
  ICON_CHECK: CheckCircle,
  ICON_WARNING: Warning,
  ICON_FILES: Files,
  ICON_PLUS: Plus,
  ICON_MINUS: Minus,
  ICON_SQUARE: Square,
  ICON_CHECK_SQUARE: CheckSquare,
  ICON_DOT: DotOutline,
  ICON_ARROW_RIGHT: ArrowRight,
  CHEVRON_DOWN: CaretDown,

  // --- Impact ---
  IMPACT_BLOCKING: HandPalm,
  IMPACT_NICE_TO_HAVE: Lightbulb,
  IMPACT_NITPICK: Microscope,
} as const;
