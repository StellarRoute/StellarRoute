Problem
useRoutes passes undefined and skip as positional args instead of the UseFetchOptions object shape useFetch expects.

Files to touch
frontend/hooks/useApi.ts (lines 199-207)
Acceptance criteria
 Skip logic works when base/quote/amount invalid
 Refresh and error toast options configurable
 Unit tests for skip true/false paths
 No spurious API calls on mount with empty pair
Contributor Prerequisites
Before starting, install and read these frontend skills:

nextjs-app-router-patterns — /Users/daniel/.claude/skills/nextjs-app-router-patterns/SKILL.md
nextjs-react-typescript — /Users/daniel/.claude/skills/nextjs-react-typescript/SKILL.md
frontend-design — /Users/daniel/.claude/skills/frontend-design/SKILL.md
Review App Router conventions used in frontend/app/ before changing routes or metadata.

Problem
usePriceHistory passes refreshIntervalMs as a bare number third argument to useFetch, but useFetch expects a UseFetchOptions object.

Files to touch
frontend/hooks/useApi.ts (lines 173-184)
Acceptance criteria
 Options passed as { refreshIntervalMs } object
 Refresh interval honored during live price polling
 skip option works when pair undefined
 Regression test prevents positional arg mistake
Contributor Prerequisites
Before starting, install and read these frontend skills:

nextjs-app-router-patterns — /Users/daniel/.claude/skills/nextjs-app-router-patterns/SKILL.md
nextjs-react-typescript — /Users/daniel/.claude/skills/nextjs-react-typescript/SKILL.md
frontend-design — /Users/daniel/.claude/skills/frontend-design/SKILL.md
Review App Router conventions used in frontend/app/ before changing routes or metadata.

Problem
usePriceHistory calls stellarRouteClient.getPriceHistory but the method is missing from lib/api/client.ts.

Files to touch
frontend/lib/api/client.ts
frontend/types/index.ts
frontend/hooks/useApi.ts
Acceptance criteria
 Client method typed against PriceHistoryResponse
 Handles pagination or window params if API supports them
 Errors surface as StellarRouteApiError
 Unit tests with mocked fetch fixtures
Contributor Prerequisites
Before starting, install and read these frontend skills:

nextjs-app-router-patterns — /Users/daniel/.claude/skills/nextjs-app-router-patterns/SKILL.md
nextjs-react-typescript — /Users/daniel/.claude/skills/nextjs-react-typescript/SKILL.md
frontend-design — /Users/daniel/.claude/skills/frontend-design/SKILL.md
Review App Router conventions used in frontend/app/ before changing routes or metadata.

Problem
Two modals exist: components/swap/TransactionConfirmationModal.tsx and components/shared/TransactionConfirmationModal.tsx with divergent props and behavior.

Files to touch
frontend/components/swap/TransactionConfirmationModal.tsx
frontend/components/shared/TransactionConfirmationModal.tsx
frontend/components/swap/SwapCard.tsx
Acceptance criteria
 Single exported modal with unified prop interface
 All imports updated; dead file removed
 Story and unit tests consolidated
 Accessibility attributes identical across previous both versions
Contributor Prerequisites
Before starting, install and read these frontend skills:

nextjs-app-router-patterns — /Users/daniel/.claude/skills/nextjs-app-router-patterns/SKILL.md
nextjs-react-typescript — /Users/daniel/.claude/skills/nextjs-react-typescript/SKILL.md
frontend-design — /Users/daniel/.claude/skills/frontend-design/SKILL.md
Review App Router conventions used in frontend/app/ before changing routes or metadata.