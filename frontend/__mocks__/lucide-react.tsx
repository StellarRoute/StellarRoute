// Minimal mock for lucide-react — used in vitest because the installed
// package ships without a compiled CJS/ESM index entry.
import * as React from 'react';

const Icon = (props: React.SVGProps<SVGSVGElement>) =>
  React.createElement('svg', { 'data-testid': 'icon', ...props });

// Icons used across the codebase
export const ArrowDown = Icon;
export const ArrowRight = Icon;
export const ArrowUp = Icon;
export const CheckCircle2 = Icon;
export const CheckIcon = Icon;
export const ChevronDown = Icon;
export const ChevronDownIcon = Icon;
export const ChevronRight = Icon;
export const ChevronRightIcon = Icon;
export const ChevronUp = Icon;
export const ChevronUpIcon = Icon;
export const CircleIcon = Icon;
export const ExternalLink = Icon;
export const Info = Icon;
export const Loader2 = Icon;
export const Menu = Icon;
export const Moon = Icon;
export const RefreshCw = Icon;
export const RefreshCcw = Icon;
export const RotateCcw = Icon;
export const Settings = Icon;
export const Home = Icon;
export const Sun = Icon;
export const Trash2 = Icon;
export const Wallet = Icon;
export const X = Icon;
export const XCircle = Icon;
export const XIcon = Icon;

// Additional icons referenced by components/tests
export const ArrowLeftRight = Icon;
export const ArrowRightLeft = Icon;
export const Check = Icon;
export const Clock = Icon;
export const Clock3 = Icon;
export const Copy = Icon;
export const Stethoscope = Icon;
export const History = Icon;
export const Search = Icon;
export const Download = Icon;

export const AlertTriangle = Icon;
export const DollarSign = Icon;
export const Layers = Icon;
export const Minus = Icon;
export const TrendingDown = Icon;
export const TrendingUp = Icon;

export const TriangleAlert = Icon;
export const AlertCircle = Icon;
export const MapPin = Icon;
export const HelpCircle = Icon;
export const ArrowUpDown = Icon;
export const Settings2 = Icon;
export const Route = Icon;

// Sonner "Toaster" icon set
export const CircleCheckIcon = Icon;
export const InfoIcon = Icon;
export const Loader2Icon = Icon;
export const OctagonXIcon = Icon;
export const TriangleAlertIcon = Icon;

// BatchSwapPreview + ViewState icons
export const Lock = Icon;
export const Inbox = Icon;
export const Plus = Icon;
export const Shield = Icon;
export const ShieldAlert = Icon;
export const Compass = Icon;
export const Zap = Icon;
export const Maximize2 = Icon;
export const Minimize2 = Icon;
export const Star = Icon;
export const Columns2 = Icon;
export const LayoutList = Icon;
export const Bell = Icon;
export const BellOff = Icon;

// Share icon
export const Share2 = Icon;

// Split icon
export const Split = Icon;

// QuoteInspector icons
export const ShieldCheck = Icon;
export const Diff = Icon;
export const Scale = Icon;
