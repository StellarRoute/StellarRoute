import { beforeEach, describe, expect, it } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';

import { CardNicknameEditor } from './CardNicknameEditor';
import {
  cardNicknameStorageKey,
  loadCardNickname,
  saveCardNickname,
} from '@/lib/card/nickname';

const ADDRESS = 'GABCDEFGHIJKLMNOPQRSTUVWXYZ234567ABCDEFGHIJKLMNOPQRSTUV2';
const OTHER_ADDRESS = 'GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB2';

beforeEach(() => {
  window.localStorage.clear();
});

describe('CardNicknameEditor', () => {
  it('shows the default title and saves an edited nickname for the connected address', () => {
    render(<CardNicknameEditor stellarAddress={ADDRESS} />);

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Stellar Virtual Debit'
    );
    fireEvent.click(screen.getByRole('button', { name: 'Edit card nickname' }));
    fireEvent.change(screen.getByLabelText('Card nickname'), {
      target: { value: 'Travel Card' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save card nickname' }));

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Travel Card'
    );
    expect(loadCardNickname(ADDRESS)).toBe('Travel Card');
    expect(
      window.localStorage.getItem(cardNicknameStorageKey(OTHER_ADDRESS))
    ).toBeNull();
  });

  it('shows inline validation for whitespace-only input', () => {
    render(<CardNicknameEditor stellarAddress={ADDRESS} />);

    fireEvent.click(screen.getByRole('button', { name: 'Edit card nickname' }));
    fireEvent.change(screen.getByLabelText('Card nickname'), {
      target: { value: '   ' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save card nickname' }));

    expect(screen.getByRole('alert')).toHaveTextContent(
      'Enter a nickname, or leave the field empty to use the default.'
    );
    expect(loadCardNickname(ADDRESS)).toBeNull();
  });

  it('strips HTML and enforces the input length when saving', () => {
    render(<CardNicknameEditor stellarAddress={ADDRESS} />);

    fireEvent.click(screen.getByRole('button', { name: 'Edit card nickname' }));
    fireEvent.change(screen.getByLabelText('Card nickname'), {
      target: { value: '<b>Travel</b>' + 'A'.repeat(30) },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save card nickname' }));

    expect(loadCardNickname(ADDRESS)).toBe('Travel' + 'A'.repeat(18));
    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Travel' + 'A'.repeat(18)
    );
  });

  it('uses the nickname stored for a newly connected Stellar address', () => {
    saveCardNickname(ADDRESS, 'Travel');
    saveCardNickname(OTHER_ADDRESS, 'Subscriptions');
    const { rerender } = render(
      <CardNicknameEditor stellarAddress={ADDRESS} />
    );

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Travel'
    );
    rerender(<CardNicknameEditor stellarAddress={OTHER_ADDRESS} />);

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Subscriptions'
    );
  });

  it('resets to the default when the input is emptied and saved', () => {
    saveCardNickname(ADDRESS, 'Travel');
    render(<CardNicknameEditor stellarAddress={ADDRESS} />);

    fireEvent.click(screen.getByRole('button', { name: 'Edit card nickname' }));
    fireEvent.change(screen.getByLabelText('Card nickname'), {
      target: { value: '' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save card nickname' }));

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Stellar Virtual Debit'
    );
    expect(loadCardNickname(ADDRESS)).toBeNull();
  });

  it('resets a saved nickname when the reset button is selected', () => {
    saveCardNickname(ADDRESS, 'Travel');
    render(<CardNicknameEditor stellarAddress={ADDRESS} />);

    fireEvent.click(screen.getByRole('button', { name: 'Edit card nickname' }));
    fireEvent.click(
      screen.getByRole('button', { name: 'Reset card nickname to default' })
    );

    expect(screen.getByTestId('card-nickname-title')).toHaveTextContent(
      'Stellar Virtual Debit'
    );
    expect(loadCardNickname(ADDRESS)).toBeNull();
  });

  it('does not allow editing without a connected Stellar address', () => {
    render(<CardNicknameEditor stellarAddress={null} />);

    expect(
      screen.getByRole('button', { name: 'Edit card nickname' })
    ).toBeDisabled();
  });
});
