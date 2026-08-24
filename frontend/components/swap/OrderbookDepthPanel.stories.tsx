import { OrderbookDepthPanel } from "./OrderbookDepthPanel";

const meta = { title: "Swap/OrderbookDepthPanel" };
export default meta;

// Stories use a real component but we can't mock hooks in Ladle easily,
// so we export named stories that document the expected states.
export const Default = () => <OrderbookDepthPanel base="XLM" quote="USDC" />;
