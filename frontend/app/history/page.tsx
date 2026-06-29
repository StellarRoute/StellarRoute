import { TransactionHistory } from "@/components/TransactionHistory";
import { Metadata } from "next";

export const metadata: Metadata = {
  title: "Transaction History | StellarRoute",
  description: "View your recent Stellar transaction history and swap activity.",
  openGraph: {
    title: "Transaction History | StellarRoute",
    description: "Track your Stellar swap history and transaction activity.",
    type: "website",
  },
};

export default function HistoryPage() {
  return (
    <div className="container mx-auto py-6">
      <TransactionHistory />
    </div>
  );
}
