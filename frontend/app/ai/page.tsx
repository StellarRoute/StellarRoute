"use client";

import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  AI_SUBSCRIPTIONS_STORAGE_KEY,
  findDueSubscription,
  parseSubscriptions,
  type AiSubscription,
} from "./subscription";

export default function AiPage() {
  const [dueSubscription, setDueSubscription] =
    useState<AiSubscription | null>(null);
  const [isDismissed, setIsDismissed] = useState(false);
  const [isConfirmed, setIsConfirmed] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    const subscriptions = parseSubscriptions(
      window.localStorage.getItem(AI_SUBSCRIPTIONS_STORAGE_KEY),
    );
    setDueSubscription(findDueSubscription(subscriptions, Date.now()));
    setIsLoaded(true);
  }, []);

  return (
    <main className="mx-auto w-full max-w-3xl px-4 py-10">
      <h1 className="text-2xl font-semibold">AI assistant</h1>

      {isLoaded && dueSubscription && !isDismissed && !isConfirmed && (
        <Card className="mt-6 max-w-xl">
          <CardHeader>
            <CardTitle>Subscription swap is due</CardTitle>
            <CardDescription>
              Review this scheduled swap and confirm before continuing. Nothing
              is signed or sent automatically.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
              <dt className="text-muted-foreground">From</dt>
              <dd>{dueSubscription.source}</dd>
              <dt className="text-muted-foreground">To</dt>
              <dd>{dueSubscription.destination}</dd>
              <dt className="text-muted-foreground">Amount</dt>
              <dd>{dueSubscription.amount}</dd>
            </dl>
            <div className="flex flex-wrap gap-2">
              <Button onClick={() => setIsConfirmed(true)}>Confirm</Button>
              <Button variant="outline" onClick={() => setIsDismissed(true)}>
                Dismiss
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {isLoaded && !dueSubscription && (
        <p className="mt-6 text-sm text-muted-foreground">
          No subscription swaps are due.
        </p>
      )}

      {isLoaded && dueSubscription && isConfirmed && (
        <p className="mt-6 text-sm" role="status">
          Confirmed for review. No transaction has been signed or sent.
        </p>
      )}
    </main>
  );
}
'use client';

import * as React from 'react';
import { AgentStatusChip } from './AgentStatusChip';
import { AgentChat } from './AgentChat';

function isAiAgentEnabled(): boolean {
  if (typeof window !== 'undefined') {
    const flags = (window as unknown as { __STELLAR_ROUTE_FLAGS__?: Record<string, boolean> })
      .__STELLAR_ROUTE_FLAGS__;
    if (flags?.ai_agent !== undefined) {
      return Boolean(flags.ai_agent);
    }
  }
  return process.env.NEXT_PUBLIC_AI_AGENT === 'true' || process.env.NEXT_PUBLIC_AI_AGENT === '1';
}

export default function AiPage() {
  const [enabled, setEnabled] = React.useState<boolean>(false);
  const [mounted, setMounted] = React.useState<boolean>(false);

  React.useEffect(() => {
    setEnabled(isAiAgentEnabled());
    setMounted(true);
  }, []);

  if (!mounted) {
    return (
      <div className="container mx-auto px-4 py-12 max-w-4xl" data-testid="ai-page-loading">
        <div className="animate-pulse space-y-4">
          <div className="h-8 w-48 bg-muted rounded" />
          <div className="h-4 w-96 bg-muted rounded" />
        </div>
      </div>
    );
  }

  if (!enabled) {
    return (
      <div className="container mx-auto px-4 py-12 max-w-4xl" data-testid="ai-page-disabled">
        <div className="rounded-lg border border-border bg-card p-8 text-center text-card-foreground shadow-sm">
          <h1 className="text-2xl font-bold tracking-tight">AI Agent Unavailable</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            The AI assistant is currently disabled on this deployment.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 py-12 max-w-4xl" data-testid="ai-page-shell">
      <div className="space-y-6">
        <div className="flex items-center justify-between pb-4 border-b border-border">
          <div>
            <h1 className="text-2xl font-bold tracking-tight">AI Agent</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Non-custodial trading assistant.
            </p>
          </div>
          <AgentStatusChip />
        </div>

        <AgentChat />
      </div>
    </div>
  );
}
