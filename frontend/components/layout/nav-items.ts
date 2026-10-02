import { ROUTES } from '@/lib/constants';

export interface NavItem {
  label: string;
  href: string;
  disabled?: boolean;
}

const BASE_NAV_ITEMS: NavItem[] = [
  { label: 'Swap', href: ROUTES.SWAP },
  { label: 'Offramp', href: ROUTES.OFFRAMP },
  { label: "Orderbook", href: "/orderbook" },
  { label: "History", href: "/history" },
];

const CARD_NAV_ITEM: NavItem = {
  label: "Card",
  href: "/card",
};

const ANALYTICS_NAV_ITEM: NavItem = {
  label: "Analytics",
  href: "/analytics",
};

const AGENT_NAV_ITEM: NavItem = {
  label: "Agent",
  href: "/ai",
};

/** Build header navigation items, optionally including analytics, agent, and card when enabled. */
export function getNavItems(options?: {
  analyticsEnabled?: boolean;
  aiAgentEnabled?: boolean;
  cardEnabled?: boolean;
}): NavItem[] {
  const items = [...BASE_NAV_ITEMS];
  if (options?.analyticsEnabled) {
    items.push(ANALYTICS_NAV_ITEM);
  }
  if (options?.aiAgentEnabled) {
    items.push(AGENT_NAV_ITEM);
  }
  if (options?.cardEnabled) {
    items.push(CARD_NAV_ITEM);
  }
  return items;
}
