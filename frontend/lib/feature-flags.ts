export type FeatureFlagName = "routesBeta";

export interface FeatureFlags {
  routesBeta: boolean;
}

type PartialFeatureFlags = Partial<FeatureFlags>;

declare global {
  interface Window {
    __STELLAR_ROUTE_FLAGS__?: PartialFeatureFlags;
  }
}

const DEFAULT_FLAGS: FeatureFlags = {
  routesBeta: false,
};

const ENV_FLAG_MAP: Record<FeatureFlagName, string> = {
  routesBeta: "NEXT_PUBLIC_FEATURE_ROUTES_BETA",
};

function parseBooleanFlag(value: string | undefined): boolean | undefined {
  if (value === undefined) {
    return undefined;
  }

  const normalized = value.trim().toLowerCase();
  if (["1", "true", "yes", "on"].includes(normalized)) {
    return true;
  }
  if (["0", "false", "no", "off"].includes(normalized)) {
    return false;
  }

  return undefined;
}

function getEnvFlags(): PartialFeatureFlags {
  const routesBeta = parseBooleanFlag(process.env[ENV_FLAG_MAP.routesBeta]);

  return routesBeta === undefined ? {} : { routesBeta };
}

function getRuntimeFlags(): PartialFeatureFlags {
  if (typeof window === "undefined") {
    return {};
  }

  return window.__STELLAR_ROUTE_FLAGS__ ?? {};
}

export function getFeatureFlags(
  overrides: PartialFeatureFlags = {},
): FeatureFlags {
  return {
    ...DEFAULT_FLAGS,
    ...getEnvFlags(),
    ...getRuntimeFlags(),
    ...overrides,
  };
}

export function getFeatureFlag(
  name: FeatureFlagName,
  overrides: PartialFeatureFlags = {},
): boolean {
  return getFeatureFlags(overrides)[name];
}
