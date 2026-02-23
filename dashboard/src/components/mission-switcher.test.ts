import { describe, expect, it } from 'vitest';

import type { Mission } from '@/lib/api';
import {
  getMissionCardDescription,
  getMissionCardTitle,
  getMissionSearchText,
  missionMatchesSearchQuery,
} from './mission-switcher';

function buildMission(overrides: Partial<Mission> = {}): Mission {
  return {
    id: 'mission-1',
    status: 'active',
    title: null,
    history: [],
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

describe('mission switcher search helpers', () => {
  it('hides short description when it is not meaningfully different from title', () => {
    const mission = buildMission({
      title: 'Fix login bug!',
      short_description: 'fix login bug',
    });

    const title = getMissionCardTitle(mission);
    const description = getMissionCardDescription(mission, title);

    expect(title).toBe('Fix login bug!');
    expect(description).toBeNull();
  });

  it('includes both title and short description in search text when each adds value', () => {
    const mission = buildMission({
      title: 'OAuth callback failures',
      short_description: 'Investigate broken login redirect URI validation',
      backend: 'claude',
      status: 'blocked',
    });

    const searchText = getMissionSearchText(mission);

    expect(searchText).toContain('OAuth callback failures');
    expect(searchText).toContain('Investigate broken login redirect URI validation');
    expect(searchText).toContain('claude');
    expect(searchText).toContain('blocked');
  });

  it('supports query expansion for auth/login style intent matching', () => {
    const mission = buildMission({
      title: 'Investigate OAuth callback failures',
      short_description: 'Auth token exchange fails after redirect',
    });

    expect(missionMatchesSearchQuery(mission, 'login callback')).toBe(true);
    expect(missionMatchesSearchQuery(mission, 'signin token')).toBe(true);
  });
});
