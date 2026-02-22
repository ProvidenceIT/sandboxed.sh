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

    /// Cached result and timestamp to avoid redundant network calls
    /// (e.g. when skip-agent-selection validates on every "New Mission" tap).
    private static var cachedData: BackendAgentData?
    private static var cacheTimestamp: Date?
    private static let cacheTTL: TimeInterval = 30 // seconds

    /// Load all enabled backends and their agents.
    /// Returns a cached result when available and fresh (within `cacheTTL`).
    static func loadBackendsAndAgents() async -> BackendAgentData {
        if let cached = cachedData,
           let ts = cacheTimestamp,
           Date().timeIntervalSince(ts) < cacheTTL {
            return cached
        }
        let data = await fetchBackendsAndAgents()
        cachedData = data
        cacheTimestamp = Date()
        return data
    }

    /// Force-reload bypassing the cache (e.g. when the user opens Settings).
    static func invalidateCache() {
        cachedData = nil
        cacheTimestamp = nil
    }

    /// Actual network fetch (extracted from the previous loadBackendsAndAgents).
    private static func fetchBackendsAndAgents() async -> BackendAgentData {
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
