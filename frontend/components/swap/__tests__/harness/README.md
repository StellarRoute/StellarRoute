# Swap Module Test Harness

This harness provides a unified testing environment for the Swap modules.

## Usage
Instead of using standard `render`, use `renderWithHarness`:

```tsx
import { renderWithHarness, screen, MOCK_ASSETS } from './harness';
import { TokenSelector } from '../TokenSelector';

test('renders tokens', () => {
  renderWithHarness(<TokenSelector assets={MOCK_ASSETS} />);
  expect(screen.getByText('XLM')).toBeInTheDocument();
});

### 🔒 Security & Integrity
When testing new swap modules, ensure you use the `MOCK_ASSETS` provided in `test-data.ts`. This ensures that all UI components are validated against the same "Stellar-standard" precision (7 decimals) to prevent rounding errors in the production UI.