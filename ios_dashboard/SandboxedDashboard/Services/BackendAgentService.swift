//
//  BackendAgentService.swift
//  SandboxedDashboard
//
//  Shared service for loading backend/agent data used across views
//

import SwiftUI

/// Result of loading backends and their agents from the API
struct BackendAgentData {
    let backends: [Backend]
    let enabledBackendIds: Set<String>
    let backendAgents: [String: [BackendAgent]]
}

/// Shared service that centralizes backend/agent loading logic
enum BackendAgentService {
    private static let api = APIService.shared

    /// Load all enabled backends and their agents
    static func loadBackendsAndAgents() async -> BackendAgentData {
        // Load backends
        let backends: [Backend]
        do {
            backends = try await api.listBackends()
        } catch {
            backends = Backend.defaults
        }

        // Load backend configs to check enabled status
        var enabled = Set<String>()
        for backend in backends {
            do {
                let config = try await api.getBackendConfig(backendId: backend.id)
                if config.isEnabled {
                    enabled.insert(backend.id)
                }
            } catch {
                // Default to enabled if we can't fetch config
                enabled.insert(backend.id)
            }
        }

        // Load agents for each enabled backend
        var backendAgents: [String: [BackendAgent]] = [:]
        for backendId in enabled {
            do {
                let agents = try await api.listBackendAgents(backendId: backendId)
                backendAgents[backendId] = agents
            } catch {
                // Use defaults for Amp if API fails
                if backendId == "amp" {
                    backendAgents[backendId] = [
                        BackendAgent(id: "smart", name: "Smart Mode"),
                        BackendAgent(id: "rush", name: "Rush Mode")
                    ]
                }
            }
        }

        return BackendAgentData(
            backends: backends,
            enabledBackendIds: enabled,
            backendAgents: backendAgents
        )
    }

    /// Icon name for a backend ID
    static func icon(for id: String?) -> String {
        switch id {
        case "opencode": return "terminal"
        case "claudecode": return "brain"
        case "amp": return "bolt.fill"
        default: return "cpu"
        }
    }

    /// Color for a backend ID
    static func color(for id: String?) -> Color {
        switch id {
        case "opencode": return Theme.success
        case "claudecode": return Theme.accent
        case "amp": return .orange
        default: return Theme.textSecondary
        }
    }
}
