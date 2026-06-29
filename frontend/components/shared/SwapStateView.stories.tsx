import { SwapStateView } from "./SwapStateView";

const meta = { title: "Shared/SwapStateView" };
export default meta;

export const QuoteLoading = () => <SwapStateView context="quote" variant="loading" />;
export const QuoteEmpty = () => <SwapStateView context="quote" variant="empty" />;
export const QuoteError = () => <SwapStateView context="quote" variant="error" onRetry={() => alert("retry")} />;

export const RoutesLoading = () => <SwapStateView context="routes" variant="loading" />;
export const RoutesEmpty = () => <SwapStateView context="routes" variant="empty" />;
export const RoutesError = () => <SwapStateView context="routes" variant="error" onRetry={() => alert("retry")} />;

export const HistoryLoading = () => <SwapStateView context="history" variant="loading" />;
export const HistoryEmpty = () => <SwapStateView context="history" variant="empty" />;
export const HistoryError = () => <SwapStateView context="history" variant="error" />;

export const WalletLoading = () => <SwapStateView context="wallet" variant="loading" />;
export const WalletEmpty = () => <SwapStateView context="wallet" variant="empty" />;
export const WalletError = () => <SwapStateView context="wallet" variant="error" />;
