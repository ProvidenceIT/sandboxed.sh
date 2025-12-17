//
//  ChatMessage.swift
//  OpenAgentDashboard
//
//  Chat message models for the control view
//

import Foundation

enum ChatMessageType {
    case user
    case assistant(success: Bool, costCents: Int, model: String?)
    case thinking(done: Bool, startTime: Date)
    case toolUI(name: String)
    case system
    case error
}

struct ChatMessage: Identifiable {
    let id: String
    let type: ChatMessageType
    var content: String
    var toolUI: ToolUIContent?
    let timestamp: Date
    
    init(id: String = UUID().uuidString, type: ChatMessageType, content: String, toolUI: ToolUIContent? = nil, timestamp: Date = Date()) {
        self.id = id
        self.type = type
        self.content = content
        self.toolUI = toolUI
        self.timestamp = timestamp
    }
    
    var isUser: Bool {
        if case .user = type { return true }
        return false
    }
    
    var isAssistant: Bool {
        if case .assistant = type { return true }
        return false
    }
    
    var isThinking: Bool {
        if case .thinking = type { return true }
        return false
    }
    
    var isToolUI: Bool {
        if case .toolUI = type { return true }
        return false
    }
    
    var thinkingDone: Bool {
        if case .thinking(let done, _) = type { return done }
        return false
    }
    
    var displayModel: String? {
        if case .assistant(_, _, let model) = type {
            if let model = model {
                return model.split(separator: "/").last.map(String.init)
            }
        }
        return nil
    }
    
    var costFormatted: String? {
        if case .assistant(_, let costCents, _) = type, costCents > 0 {
            return String(format: "$%.4f", Double(costCents) / 100.0)
        }
        return nil
    }
}

// MARK: - Control Session State

enum ControlRunState: String, Codable {
    case idle
    case running
    case waitingForTool = "waiting_for_tool"
    
    var statusType: StatusType {
        switch self {
        case .idle: return .idle
        case .running: return .running
        case .waitingForTool: return .pending
        }
    }
    
    var label: String {
        switch self {
        case .idle: return "Idle"
        case .running: return "Running"
        case .waitingForTool: return "Waiting"
        }
    }
}
