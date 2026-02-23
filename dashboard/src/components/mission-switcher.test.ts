import { describe, expect, it } from 'vitest';

import type { Mission } from '@/lib/api';
import {
  getMissionCardDescription,
  getMissionCardTitle,
  getMissionSearchText,
  missionMatchesSearchQuery,
  missionSearchRelevanceScore,
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

  it('uses the real default backend label when backend is missing', () => {
    const mission = buildMission({
      title: 'Investigate OAuth callback failures',
      short_description: 'Auth token exchange fails after redirect',
      backend: undefined,
    });

    const searchText = getMissionSearchText(mission);
    expect(searchText).toContain('claudecode');
  });

  it('supports query expansion for auth/login style intent matching', () => {
    const mission = buildMission({
      title: 'Investigate OAuth callback failures',
      short_description: 'Auth token exchange fails after redirect',
    });

    expect(missionMatchesSearchQuery(mission, 'login callback')).toBe(true);
    expect(missionMatchesSearchQuery(mission, 'signin token')).toBe(true);
  });

  it('avoids false positives for very short query prefixes', () => {
    const mission = buildMission({
      title: 'Authentication callback refactor',
      short_description: 'Improve OAuth and credential handling',
    });

    expect(missionMatchesSearchQuery(mission, 'a')).toBe(false);
    expect(missionSearchRelevanceScore(mission, 'a')).toBe(0);
  });

  it('still matches common inflections like timeout/timeouts', () => {
    const mission = buildMission({
      title: 'Handle timeout retries for session refresh',
    });

    expect(missionMatchesSearchQuery(mission, 'timeouts')).toBe(true);
    expect(missionSearchRelevanceScore(mission, 'timeouts')).toBeGreaterThan(0);
  });

  it('ranks exact title phrase matches above weaker synonym matches', () => {
    const exactMission = buildMission({
      title: 'Login timeout when refreshing session',
      short_description: 'Timeout occurs after token refresh',
      updated_at: '2026-01-10T00:00:00Z',
    });
    const synonymMission = buildMission({
      id: 'mission-2',
      title: 'Authentication latency investigation',
      short_description: 'Investigate slow oauth callback exchanges',
      updated_at: '2026-01-11T00:00:00Z',
    });

    const query = 'login timeout';
    const exactScore = missionSearchRelevanceScore(exactMission, query);
    const synonymScore = missionSearchRelevanceScore(synonymMission, query);

    expect(exactScore).toBeGreaterThan(synonymScore);
    expect(exactScore).toBeGreaterThan(0);
    expect(synonymScore).toBeGreaterThan(0);
  });

  it('keeps non-matching missions at zero relevance', () => {
    const mission = buildMission({
      title: 'Refactor CSS variables',
      short_description: 'Tighten spacing and typography in dashboard',
    });

    expect(missionSearchRelevanceScore(mission, 'database migration')).toBe(0);
    expect(missionMatchesSearchQuery(mission, 'database migration')).toBe(false);
  });
});
