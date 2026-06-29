/**
 * OpenAPI contract validation tests (#759)
 *
 * Validates that fixture responses — representative of real recorded API
 * responses — conform to the OpenAPI schema definitions in
 * `frontend/schemas/openapi.json`.
 *
 * Test categories:
 *   1. Structural — schema paths and component schemas exist
 *   2. Positive fixture validation — fixtures pass schema validation
 *   3. Negative / contract-drift — mutated fixtures fail schema validation
 *      (ensures CI breaks on breaking API changes)
 */

import Ajv from "ajv";
import addFormats from "ajv-formats";
import { describe, test, expect, beforeAll } from "vitest";

import openapiSchema from "../schemas/openapi.json";

// ── Fixtures (recorded representative API responses) ─────────────────────────
import pairsFixture from "./fixtures/pairs-response.json";
import orderbookFixture from "./fixtures/orderbook-response.json";
import quoteFixture from "./fixtures/quote-response.json";
import routesFixture from "./fixtures/routes-response.json";
import healthFixture from "./fixtures/health-response.json";
import errorFixture from "./fixtures/error-response.json";

// ── AJV setup ────────────────────────────────────────────────────────────────
// strict: false allows the nullable extension used by OpenAPI 3.0 schemas.
// allErrors: true gives full error lists for easier debugging.
let ajv: Ajv;

beforeAll(() => {
  ajv = new Ajv({ strict: false, allErrors: true });
  addFormats(ajv);
  // Pre-register the entire components.schemas block so $ref resolution works
  // across all named schemas.
  Object.entries(openapiSchema.components.schemas).forEach(([name, schema]) => {
    ajv.addSchema(schema as object, `#/components/schemas/${name}`);
  });
});

// ── Helper ───────────────────────────────────────────────────────────────────
function validateAgainst(schemaRef: string, data: unknown): { valid: boolean; errors: string } {
  const validate = ajv.getSchema(schemaRef);
  if (!validate) {
    throw new Error(`Schema not found: ${schemaRef}`);
  }
  const valid = validate(data) as boolean;
  const errors = valid ? "" : JSON.stringify(validate.errors, null, 2);
  return { valid, errors };
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Structural tests — paths and component schemas must exist
// ─────────────────────────────────────────────────────────────────────────────
describe("OpenAPI schema structure", () => {
  const requiredPaths = [
    "/health",
    "/api/v1/pairs",
    "/api/v1/orderbook/{base}/{quote}",
    "/api/v1/quote/{base}/{quote}",
    "/api/v1/routes/{base}/{quote}",
  ] as const;

  requiredPaths.forEach((path) => {
    test(`path "${path}" exists`, () => {
      expect(openapiSchema.paths).toHaveProperty(path);
    });
  });

  const requiredSchemas = [
    "PairsResponse",
    "OrderbookResponse",
    "QuoteResponse",
    "RoutesResponse",
    "RouteCandidate",
    "RouteHop",
    "HealthResponse",
    "ErrorResponse",
    "AssetInfo",
    "TradingPair",
    "PathStep",
    "OrderbookLevel",
  ] as const;

  requiredSchemas.forEach((name) => {
    test(`component schema "${name}" exists`, () => {
      expect(openapiSchema.components.schemas).toHaveProperty(name);
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Positive fixture validation — all recorded responses must pass
// ─────────────────────────────────────────────────────────────────────────────
describe("Fixture validation — positive path", () => {
  test("pairs-response.json conforms to PairsResponse schema", () => {
    const { valid, errors } = validateAgainst(
      "#/components/schemas/PairsResponse",
      pairsFixture,
    );
    expect(valid, `PairsResponse validation errors:\n${errors}`).toBe(true);
  });

  test("orderbook-response.json conforms to OrderbookResponse schema", () => {
    const { valid, errors } = validateAgainst(
      "#/components/schemas/OrderbookResponse",
      orderbookFixture,
    );
    expect(valid, `OrderbookResponse validation errors:\n${errors}`).toBe(true);
  });

  test("quote-response.json conforms to QuoteResponse schema", () => {
    const { valid, errors } = validateAgainst(
      "#/components/schemas/QuoteResponse",
      quoteFixture,
    );
    expect(valid, `QuoteResponse validation errors:\n${errors}`).toBe(true);
  });

  test("routes-response.json conforms to RoutesResponse schema", () => {
    const { valid, errors } = validateAgainst(
      "#/components/schemas/RoutesResponse",
      routesFixture,
    );
    expect(valid, `RoutesResponse validation errors:\n${errors}`).toBe(true);
  });

  test("health-response.json conforms to HealthResponse schema", () => {
    const { valid, errors } = validateAgainst(
      "#/components/schemas/HealthResponse",
      healthFixture,
    );
    expect(valid, `HealthResponse validation errors:\n${errors}`).toBe(true);
  });

  test("error-response.json conforms to ErrorResponse schema", () => {
    const { valid, errors } = validateAgainst(
      "#/components/schemas/ErrorResponse",
      errorFixture,
    );
    expect(valid, `ErrorResponse validation errors:\n${errors}`).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. Detailed field assertions — sanity-check fixture shape beyond schema
// ─────────────────────────────────────────────────────────────────────────────
describe("Fixture field assertions", () => {
  test("PairsResponse: contains at least one pair with required fields", () => {
    const fixture = pairsFixture as {
      pairs: { base: string; counter: string; base_asset: string; counter_asset: string; offer_count: number }[];
      total: number;
    };
    expect(fixture.pairs.length).toBeGreaterThan(0);
    expect(fixture.total).toBe(fixture.pairs.length);
    const first = fixture.pairs[0];
    expect(typeof first.base).toBe("string");
    expect(typeof first.counter).toBe("string");
    expect(typeof first.offer_count).toBe("number");
  });

  test("OrderbookResponse: bids are sorted highest-price-first", () => {
    const fixture = orderbookFixture as {
      bids: { price: string }[];
      asks: { price: string }[];
    };
    const bidPrices = fixture.bids.map((b) => parseFloat(b.price));
    for (let i = 1; i < bidPrices.length; i++) {
      expect(bidPrices[i]).toBeLessThanOrEqual(bidPrices[i - 1]);
    }
  });

  test("OrderbookResponse: asks are sorted lowest-price-first", () => {
    const fixture = orderbookFixture as { asks: { price: string }[] };
    const askPrices = fixture.asks.map((a) => parseFloat(a.price));
    for (let i = 1; i < askPrices.length; i++) {
      expect(askPrices[i]).toBeGreaterThanOrEqual(askPrices[i - 1]);
    }
  });

  test("QuoteResponse: path steps reference valid source values", () => {
    const fixture = quoteFixture as { path: { source: string }[] };
    for (const step of fixture.path) {
      expect(step.source === "sdex" || step.source.startsWith("amm:")).toBe(true);
    }
  });

  test("RoutesResponse: routes are in descending score order", () => {
    const fixture = routesFixture as { routes: { score: number }[] };
    expect(fixture.routes.length).toBeGreaterThan(0);
    for (let i = 1; i < fixture.routes.length; i++) {
      expect(fixture.routes[i].score).toBeLessThanOrEqual(fixture.routes[i - 1].score);
    }
  });

  test("RoutesResponse: every route has at least one hop", () => {
    const fixture = routesFixture as { routes: { path: unknown[] }[] };
    for (const route of fixture.routes) {
      expect(route.path.length).toBeGreaterThan(0);
    }
  });

  test("RoutesResponse: multi-hop route has from/to asset continuity", () => {
    const fixture = routesFixture as {
      routes: {
        path: { from_asset: { asset_code: string | null }; to_asset: { asset_code: string | null } }[];
      }[];
    };
    const multiHop = fixture.routes.find((r) => r.path.length > 1);
    if (!multiHop) return; // skip if no multi-hop route in fixture
    for (let i = 1; i < multiHop.path.length; i++) {
      // The to_asset of hop N should equal the from_asset of hop N+1
      expect(multiHop.path[i].from_asset.asset_code).toBe(
        multiHop.path[i - 1].to_asset.asset_code,
      );
    }
  });

  test("ErrorResponse: has machine-readable error code and human message", () => {
    const fixture = errorFixture as { error: string; message: string };
    expect(typeof fixture.error).toBe("string");
    expect(fixture.error.length).toBeGreaterThan(0);
    expect(typeof fixture.message).toBe("string");
    expect(fixture.message.length).toBeGreaterThan(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Negative / contract-drift tests — mutated payloads must fail validation
//    These tests ensure CI will break when the API removes a required field.
// ─────────────────────────────────────────────────────────────────────────────
describe("Contract-drift detection — negative path", () => {
  test("PairsResponse: missing 'pairs' field fails validation", () => {
    const { pairs: _omit, ...broken } = pairsFixture as { pairs: unknown; total: number };
    const { valid } = validateAgainst("#/components/schemas/PairsResponse", broken);
    expect(valid).toBe(false);
  });

  test("PairsResponse: missing 'total' field fails validation", () => {
    const { total: _omit, ...broken } = pairsFixture as { pairs: unknown; total: number };
    const { valid } = validateAgainst("#/components/schemas/PairsResponse", broken);
    expect(valid).toBe(false);
  });

  test("OrderbookResponse: missing 'bids' field fails validation", () => {
    const { bids: _omit, ...broken } = orderbookFixture as { bids: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/OrderbookResponse", broken);
    expect(valid).toBe(false);
  });

  test("OrderbookResponse: missing 'timestamp' field fails validation", () => {
    const { timestamp: _omit, ...broken } = orderbookFixture as { timestamp: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/OrderbookResponse", broken);
    expect(valid).toBe(false);
  });

  test("QuoteResponse: missing 'amount' field fails validation", () => {
    const { amount: _omit, ...broken } = quoteFixture as { amount: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/QuoteResponse", broken);
    expect(valid).toBe(false);
  });

  test("QuoteResponse: missing 'path' field fails validation", () => {
    const { path: _omit, ...broken } = quoteFixture as { path: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/QuoteResponse", broken);
    expect(valid).toBe(false);
  });

  test("QuoteResponse: missing 'quote_type' field fails validation", () => {
    const { quote_type: _omit, ...broken } = quoteFixture as { quote_type: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/QuoteResponse", broken);
    expect(valid).toBe(false);
  });

  test("RoutesResponse: missing 'routes' array fails validation", () => {
    const { routes: _omit, ...broken } = routesFixture as { routes: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/RoutesResponse", broken);
    expect(valid).toBe(false);
  });

  test("RoutesResponse: route missing required 'score' fails validation", () => {
    const brokenRoutes = (routesFixture as { routes: { score: unknown; [k: string]: unknown }[] }).routes.map(
      ({ score: _omit, ...rest }) => rest,
    );
    const broken = { ...(routesFixture as object), routes: brokenRoutes };
    const { valid } = validateAgainst("#/components/schemas/RoutesResponse", broken);
    expect(valid).toBe(false);
  });

  test("RoutesResponse: route missing required 'path' fails validation", () => {
    const brokenRoutes = (routesFixture as { routes: { path: unknown; [k: string]: unknown }[] }).routes.map(
      ({ path: _omit, ...rest }) => rest,
    );
    const broken = { ...(routesFixture as object), routes: brokenRoutes };
    const { valid } = validateAgainst("#/components/schemas/RoutesResponse", broken);
    expect(valid).toBe(false);
  });

  test("HealthResponse: missing 'status' field fails validation", () => {
    const { status: _omit, ...broken } = healthFixture as { status: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/HealthResponse", broken);
    expect(valid).toBe(false);
  });

  test("HealthResponse: invalid 'status' enum value fails validation", () => {
    const broken = { ...(healthFixture as object), status: "degraded" };
    const { valid } = validateAgainst("#/components/schemas/HealthResponse", broken);
    expect(valid).toBe(false);
  });

  test("ErrorResponse: missing 'error' field fails validation", () => {
    const { error: _omit, ...broken } = errorFixture as { error: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/ErrorResponse", broken);
    expect(valid).toBe(false);
  });

  test("ErrorResponse: missing 'message' field fails validation", () => {
    const { message: _omit, ...broken } = errorFixture as { message: unknown; [k: string]: unknown };
    const { valid } = validateAgainst("#/components/schemas/ErrorResponse", broken);
    expect(valid).toBe(false);
  });

  test("AssetInfo: invalid asset_type enum value fails validation", () => {
    const brokenAsset = {
      asset_type: "credit_alphanum99", // not in enum
      asset_code: "FOO",
      asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
    };
    const { valid } = validateAgainst("#/components/schemas/AssetInfo", brokenAsset);
    expect(valid).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. Edge case coverage
// ─────────────────────────────────────────────────────────────────────────────
describe("Edge case coverage", () => {
  test("RoutesResponse: empty routes array is valid", () => {
    const emptyRoutes = {
      base_asset: { asset_type: "native", asset_code: null, asset_issuer: null },
      quote_asset: {
        asset_type: "credit_alphanum4",
        asset_code: "USDC",
        asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      },
      amount: "100.0000000",
      timestamp: 1740312000,
      routes: [],
    };
    const { valid, errors } = validateAgainst("#/components/schemas/RoutesResponse", emptyRoutes);
    expect(valid, `Unexpected errors:\n${errors}`).toBe(true);
  });

  test("PairsResponse: pair with optional last_updated omitted is valid", () => {
    const fixture = {
      pairs: [
        {
          base: "XLM",
          counter: "USDC",
          base_asset: "native",
          counter_asset: "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
          offer_count: 5,
          // last_updated intentionally omitted — it is nullable/optional
        },
      ],
      total: 1,
    };
    const { valid, errors } = validateAgainst("#/components/schemas/PairsResponse", fixture);
    expect(valid, `Unexpected errors:\n${errors}`).toBe(true);
  });

  test("OrderbookResponse: empty bids and asks arrays are valid", () => {
    const fixture = {
      base_asset: { asset_type: "native", asset_code: null, asset_issuer: null },
      quote_asset: {
        asset_type: "credit_alphanum4",
        asset_code: "USDC",
        asset_issuer: "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      },
      bids: [],
      asks: [],
      timestamp: 1740312000,
    };
    const { valid, errors } = validateAgainst("#/components/schemas/OrderbookResponse", fixture);
    expect(valid, `Unexpected errors:\n${errors}`).toBe(true);
  });

  test("HealthResponse: redis not_configured is a valid component status", () => {
    const fixture = {
      status: "healthy",
      timestamp: "2026-02-23T12:00:00Z",
      version: "0.1.0",
      components: { database: "healthy", redis: "not_configured" },
    };
    const { valid, errors } = validateAgainst("#/components/schemas/HealthResponse", fixture);
    expect(valid, `Unexpected errors:\n${errors}`).toBe(true);
  });

  test("HealthResponse: unhealthy status is valid", () => {
    const fixture = {
      status: "unhealthy",
      timestamp: "2026-02-23T12:00:00Z",
      version: "0.1.0",
      components: { database: "unhealthy", redis: "healthy" },
    };
    const { valid, errors } = validateAgainst("#/components/schemas/HealthResponse", fixture);
    expect(valid, `Unexpected errors:\n${errors}`).toBe(true);
  });
});