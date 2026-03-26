// Minimal mock for lucide-react — used in vitest because the installed
// package ships without a compiled CJS/ESM index entry.
import * as React from "react";

const Icon = (props: React.SVGProps<SVGSVGElement>) =>
  React.createElement("svg", { "data-testid": "icon", ...props });

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
export const Settings = Icon;
export const Sun = Icon;
export const Trash2 = Icon;
export const Wallet = Icon;
export const X = Icon;
export const XCircle = Icon;
export const XIcon = Icon;
