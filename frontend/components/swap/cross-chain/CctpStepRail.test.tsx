import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { CctpStepRail } from './CctpStepRail';

describe('CctpStepRail', () => {
  it('labels preview steps without implying live execution', () => {
    render(<CctpStepRail previewOnly />);
    expect(screen.getByLabelText('CCTP protocol steps')).toBeInTheDocument();
    expect(screen.getAllByText(/preview — not live/i).length).toBeGreaterThan(0);
    expect(screen.getByText('Burn')).toBeInTheDocument();
    expect(screen.getByText('Attest')).toBeInTheDocument();
    expect(screen.getByText('Mint')).toBeInTheDocument();
  });

  it('marks completed hops when live status is completed', () => {
    render(
      <CctpStepRail
        previewOnly={false}
        activeStep={null}
        completedSteps={['burn', 'attest', 'mint']}
      />,
    );
    expect(screen.getAllByText(/complete/i).length).toBeGreaterThanOrEqual(3);
  });
});
