'use client';

import dynamic from 'next/dynamic';

// Lazy-load route panel chunk to improve swap page TTI.
// `RouteDisplay` is client-only and already uses virtualization.
const RouteDisplay = dynamic(
  () => import('./RouteDisplay').then((m) => m.RouteDisplay),
  {
    ssr: false,
  }
);

export default RouteDisplay;
