/**
 * Automations API - scheduled command triggers for missions.
 */

import { apiFetch, apiGet, apiPatch, apiDel } from "./core";

export interface Automation {
  id: string;
  mission_id: string;
  command_name: string;
  interval_seconds: number;
  active: boolean;
  created_at: string;
  last_triggered_at?: string | null;
}

export async function listMissionAutomations(missionId: string): Promise<Automation[]> {
  return apiGet(`/api/control/missions/${missionId}/automations`, "Failed to fetch automations");
}

export async function createMissionAutomation(
  missionId: string,
  input: { commandName: string; intervalSeconds: number }
): Promise<Automation> {
  const res = await apiFetch(`/api/control/missions/${missionId}/automations`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      command_name: input.commandName,
      interval_seconds: input.intervalSeconds,
    }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(text || "Failed to create automation");
  }
  return res.json();
}

export async function getAutomation(automationId: string): Promise<Automation> {
  return apiGet(`/api/control/automations/${automationId}`, "Failed to fetch automation");
}

export async function updateAutomationActive(
  automationId: string,
  active: boolean
): Promise<Automation> {
  return apiPatch(
    `/api/control/automations/${automationId}`,
    { active },
    "Failed to update automation"
  );
}

export async function deleteAutomation(automationId: string): Promise<void> {
  await apiDel(`/api/control/automations/${automationId}`, "Failed to delete automation");
}
