export function useRouter() {
  return { push: () => {}, replace: () => {}, back: () => {} };
}

export function useSearchParams() {
  return new URLSearchParams();
}

export function usePathname() {
  return '/swap';
}
