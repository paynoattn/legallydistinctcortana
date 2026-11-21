import type { ReactElement, SVGProps } from 'react';

type PathStyle = SVGProps<SVGPathElement>;

const FILLED: PathStyle = { fill: 'currentColor' };
const LINED: PathStyle = {
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.8,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
};

interface IconProps {
  readonly d: string;
  readonly lined?: boolean;
}

export function Icon({ d, lined = false }: IconProps): ReactElement {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d={d} {...(lined ? LINED : FILLED)} />
    </svg>
  );
}
