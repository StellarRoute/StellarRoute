import { useEffect, useState } from 'react';

const ONBOARDING_SEEN_KEY = 'stellarroute.onboarding.seen';
const ONBOARDING_COMPLETED_KEY = 'stellarroute.onboarding.completed';

interface UseWalletOnboardingOptions {
  isConnected: boolean;
}

export function useWalletOnboarding({ isConnected }: UseWalletOnboardingOptions) {
  const [showOnboarding, setShowOnboarding] = useState(false);
  const [hasSeenOnboarding, setHasSeenOnboarding] = useState(false);
  const [isFirstConnection, setIsFirstConnection] = useState(false);

  useEffect(() => {
    if (typeof window === 'undefined') return;

    // Check if user has completed onboarding
    const completed = window.localStorage.getItem(ONBOARDING_COMPLETED_KEY);
    const seen = window.localStorage.getItem(ONBOARDING_SEEN_KEY);

    // Show onboarding if:
    // 1. User is not connected yet AND
    // 2. User hasn't seen the onboarding flow yet
    const shouldShowOnboarding = !isConnected && !seen;
    
    setIsFirstConnection(!isConnected && !completed);
    setHasSeenOnboarding(!!seen);
    setShowOnboarding(shouldShowOnboarding);
  }, [isConnected]);

  const markOnboardingAsSeenAndOpened = () => {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(ONBOARDING_SEEN_KEY, 'true');
  };

  const markOnboardingAsCompleted = () => {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(ONBOARDING_COMPLETED_KEY, 'true');
    window.localStorage.setItem(ONBOARDING_SEEN_KEY, 'true');
  };

  const resetOnboarding = () => {
    if (typeof window === 'undefined') return;
    window.localStorage.removeItem(ONBOARDING_SEEN_KEY);
    window.localStorage.removeItem(ONBOARDING_COMPLETED_KEY);
    setShowOnboarding(true);
    setIsFirstConnection(true);
    setHasSeenOnboarding(false);
  };

  return {
    showOnboarding,
    isFirstConnection,
    hasSeenOnboarding,
    markOnboardingAsSeenAndOpened,
    markOnboardingAsCompleted,
    resetOnboarding,
  };
}
