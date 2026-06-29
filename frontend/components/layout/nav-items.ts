export interface NavItem {
  label: string;
  href: string;
  disabled?: boolean;
}

const BASE_NAV_ITEMS: NavItem[] = [
  { label: "Swap", href: "/swap" },
  { label: "Orderbook", href: "/orderbook" },
  { label: "History", href: "/history" },
];

const ANALYTICS_NAV_ITEM: NavItem = {
  label: "Analytics",
  href: "/analytics",
};

/** Build header navigation items, optionally including analytics when enabled. */
export function getNavItems(options: { analyticsEnabled: boolean }): NavItem[] {
  if (!options.analyticsEnabled) {
    return BASE_NAV_ITEMS;
  }

  return [...BASE_NAV_ITEMS, ANALYTICS_NAV_ITEM];
}
