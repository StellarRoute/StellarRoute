import { ViewState, LoadingState, ErrorState, EmptyState } from "./ViewState";
import { Button } from "@/components/ui/button";

const meta = {
  title: "Shared/ViewState",
  component: ViewState,
  parameters: {
    layout: "centered",
  },
  argTypes: {
    variant: {
      control: "select",
      options: ["loading", "empty", "error"],
      description: "Visual variant of the state",
    },
    title: { control: "text" },
    description: { control: "text" },
  },
};

export default meta;

export const Loading = () => (
  <ViewState variant="loading" title="Loading quotes..." description="Fetching the best prices for you" />
);

export const LoadingWithCustomMessage = () => (
  <ViewState variant="loading" title="Finding routes..." description="This may take a few seconds" />
);

export const LoadingStateComponent = () => (
  <LoadingState message="Loading your portfolio..." />
);

export const Empty = () => (
  <ViewState
    variant="empty"
    title="No quotes available"
    description="Try adjusting your search or check back later"
  />
);

export const EmptyWithAction = () => (
  <ViewState
    variant="empty"
    title="No trading pairs found"
    description="Add a token to your watchlist to get started"
    action={
      <Button variant="outline" size="sm" onClick={() => alert("Add token clicked")}>
        Add Token
      </Button>
    }
  />
);

export const EmptyStateComponent = () => (
  <EmptyState
    message="No history yet"
    description="Your swap history will appear here after your first trade"
    action={
      <Button variant="outline" size="sm" onClick={() => alert("Start trading clicked")}>
        Start Trading
      </Button>
    }
  />
);

export const Error = () => (
  <ViewState
    variant="error"
    title="Something went wrong"
    description="Unable to fetch quotes at this time. Please try again."
  />
);

export const ErrorWithRetry = () => (
  <ViewState
    variant="error"
    title="Connection failed"
    description="Check your internet connection and try again"
    action={
      <Button variant="outline" size="sm" onClick={() => alert("Retry clicked")}>
        Retry
      </Button>
    }
  />
);

export const ErrorStateComponent = () => (
  <ErrorState
    message="Failed to load orderbook"
    onRetry={() => alert("Retry clicked")}
  />
);

export const AllVariants = () => (
  <div className="grid grid-cols-1 md:grid-cols-3 gap-6 w-full max-w-4xl">
    <ViewState variant="loading" title="Loading" description="Please wait..." />
    <ViewState variant="empty" title="Empty" description="Nothing here yet" />
    <ViewState
      variant="error"
      title="Error"
      description="Something failed"
      action={<Button variant="outline" size="sm">Retry</Button>}
    />
  </div>
);