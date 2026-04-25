## Learned User Preferences
- Prefer autonomous, sequential execution with minimal back-and-forth; avoid asking for clarification and just proceed.
- When fixing CI, use the failing logs as a feedback loop and keep iterating until all checks are green.
- While fixing CI, regularly pull latest changes to ensure working on real/up-to-date files.
- When creating GitHub issues for this project, include a complexity rating/label and clear acceptance criteria (Wave-ready).

## Learned Workspace Facts
- Frontend tests use Vitest; jsdom lacks `window.matchMedia`, so tests may require a mock in `frontend/vitest.setup.ts` wired via `frontend/vitest.config.ts`.
- `frontend/__mocks__/lucide-react.tsx` is used for icon mocking; missing icon exports can cause React component tests to fail with “Element type is invalid … got undefined”.
