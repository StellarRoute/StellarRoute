import { describe, it, expect, beforeEach } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { clearCap, exceedsCap, loadCap, saveCap, usdcEquivalent } from './cap';

describe('spending cap storage', () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it('is unset by default', () => {
    expect(loadCap()).toBeNull();
  });

  it('saves and loads a positive cap', () => {
    expect(saveCap('50')).toBe(50);
    expect(loadCap()).toBe(50);
  });

  it('treats empty, zero, or invalid input as no cap', () => {
    saveCap('50');
    expect(saveCap('')).toBeNull();
    expect(loadCap()).toBeNull();
    expect(saveCap('0')).toBeNull();
    expect(saveCap('abc')).toBeNull();
    saveCap('50');
    clearCap();
    expect(loadCap()).toBeNull();
  });
});

describe('exceedsCap', () => {
  it('unset cap never blocks', () => {
    expect(exceedsCap('1000000', 'USDC', null)).toBe(false);
  });

  it('blocks a USDC amount above the cap', () => {
    expect(exceedsCap('100.5', 'USDC', 100)).toBe(true);
    expect(exceedsCap('100.5', 'USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN', 100)).toBe(true);
  });

  it('allows an amount within the cap', () => {
    expect(exceedsCap('100', 'usdc', 100)).toBe(false);
    expect(exceedsCap('5', 'USDC', 100)).toBe(false);
  });

  it('uses a USDC rate for other assets and does not block when unknown', () => {
    expect(usdcEquivalent('100', 'XLM', { XLM: 0.1 })).toBeCloseTo(10);
    expect(exceedsCap('2000', 'XLM', 100, { XLM: 0.1 })).toBe(true);
    expect(exceedsCap('2000', 'XLM', 100)).toBe(false);
  });
});

describe('/swap ignores the cap', () => {
  const root = resolve(__dirname, '../..');

  function sources(dir: string): string[] {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return sources(path);
      return /\.(ts|tsx)$/.test(name) && !/\.test\.tsx?$/.test(name) ? [path] : [];
    });
  }

  it('no /swap source imports the cap module', () => {
    const files = [...sources(join(root, 'app/swap')), ...sources(join(root, 'components/swap'))];
    expect(files.length).toBeGreaterThan(0);
    for (const file of files) {
      expect(readFileSync(file, 'utf8'), file).not.toMatch(/lib\/ai\/cap/);
    }
  });
});
