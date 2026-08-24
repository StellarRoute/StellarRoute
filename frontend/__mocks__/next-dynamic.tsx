import type { ComponentType } from 'react';

export default function dynamic<P extends object>(
  loader: () => Promise<{ default: ComponentType<P> } | ComponentType<P>>,
) {
  let Component: ComponentType<P> | null = null;
  loader().then((mod) => {
    Component = ('default' in mod ? mod.default : mod) as ComponentType<P>;
  });

  return function DynamicComponent(props: P) {
    if (!Component) {
      return null;
    }
    return <Component {...props} />;
  };
}
