//
//  ToolUIView.swift
//  OpenAgentDashboard
//
//  Main renderer for tool UI components
//

import SwiftUI

struct ToolUIView: View {
    let content: ToolUIContent
    let onOptionSelect: ((String, String) -> Void)?
    
    init(content: ToolUIContent, onOptionSelect: ((String, String) -> Void)? = nil) {
        self.content = content
        self.onOptionSelect = onOptionSelect
    }
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Tool label
            HStack(spacing: 6) {
                Image(systemName: toolIcon)
                    .font(.caption2)
                    .foregroundStyle(Theme.accent)
                
                Text("Tool: ")
                    .font(.caption2)
                    .foregroundStyle(Theme.textTertiary)
                +
                Text(toolName)
                    .font(.caption2.monospaced())
                    .foregroundStyle(Theme.accent)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 6)
            .background(Theme.accent.opacity(0.1))
            .clipShape(Capsule())
            
            // Tool content
            toolContent
        }
    }
    
    @ViewBuilder
    private var toolContent: some View {
        switch content {
        case .dataTable(let table):
            ToolUIDataTableView(table: table)
            
        case .optionList(let list):
            ToolUIOptionListView(optionList: list) { optionId in
                if let listId = list.id {
                    onOptionSelect?(listId, optionId)
                }
            }
            
        case .unknown(let name, let args):
            unknownToolView(name: name, args: args)
        }
    }
    
    private var toolIcon: String {
        switch content {
        case .dataTable:
            return "tablecells"
        case .optionList:
            return "list.bullet"
        case .unknown:
            return "questionmark.circle"
        }
    }
    
    private var toolName: String {
        switch content {
        case .dataTable:
            return "ui_dataTable"
        case .optionList:
            return "ui_optionList"
        case .unknown(let name, _):
            return name
        }
    }
    
    private func unknownToolView(name: String, args: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Unknown Tool UI")
                .font(.subheadline.weight(.medium))
                .foregroundStyle(Theme.textSecondary)
            
            Text(args)
                .font(.caption.monospaced())
                .foregroundStyle(Theme.textTertiary)
                .lineLimit(10)
        }
        .padding(12)
        .background(.ultraThinMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(Theme.border, lineWidth: 0.5)
        )
    }
}

#Preview {
    VStack(spacing: 20) {
        Text("Tool UI Preview")
    }
    .padding()
    .background(Theme.backgroundPrimary)
}
