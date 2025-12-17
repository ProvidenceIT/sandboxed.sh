//
//  ToolUIDataTableView.swift
//  OpenAgentDashboard
//
//  SwiftUI renderer for ui_dataTable tool
//

import SwiftUI

struct ToolUIDataTableView: View {
    let table: ToolUIDataTable
    
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Title
            if let title = table.title {
                Text(title)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(Theme.textPrimary)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Theme.backgroundSecondary.opacity(0.5))
            }
            
            // Table content
            ScrollView(.horizontal, showsIndicators: false) {
                VStack(alignment: .leading, spacing: 0) {
                    // Header row
                    HStack(spacing: 0) {
                        ForEach(table.columns, id: \.id) { column in
                            Text(column.displayLabel)
                                .font(.caption.weight(.semibold))
                                .foregroundStyle(Theme.textTertiary)
                                .textCase(.uppercase)
                                .frame(minWidth: columnWidth(for: column), alignment: .leading)
                                .padding(.horizontal, 12)
                                .padding(.vertical, 10)
                        }
                    }
                    .background(Theme.backgroundSecondary.opacity(0.3))
                    
                    Divider()
                        .background(Theme.border)
                    
                    // Data rows
                    if table.rows.isEmpty {
                        Text("No data")
                            .font(.subheadline)
                            .foregroundStyle(Theme.textMuted)
                            .padding()
                            .frame(maxWidth: .infinity, alignment: .center)
                    } else {
                        ForEach(Array(table.rows.enumerated()), id: \.offset) { index, row in
                            HStack(spacing: 0) {
                                ForEach(table.columns, id: \.id) { column in
                                    let cellValue = row[column.id]?.stringValue ?? "-"
                                    Text(cellValue)
                                        .font(.subheadline)
                                        .foregroundStyle(Theme.textSecondary)
                                        .frame(minWidth: columnWidth(for: column), alignment: .leading)
                                        .padding(.horizontal, 12)
                                        .padding(.vertical, 10)
                                }
                            }
                            
                            if index < table.rows.count - 1 {
                                Divider()
                                    .background(Theme.border.opacity(0.5))
                            }
                        }
                    }
                }
            }
        }
        .background(.ultraThinMaterial)
        .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 14, style: .continuous)
                .stroke(Theme.border, lineWidth: 0.5)
        )
    }
    
    private func columnWidth(for column: ToolUIDataTable.Column) -> CGFloat {
        // Parse width if provided, otherwise use default
        if let width = column.width {
            if width.hasSuffix("px") {
                let numStr = width.dropLast(2)
                if let num = Double(numStr) {
                    return CGFloat(num)
                }
            }
            if let num = Double(width) {
                return CGFloat(num)
            }
        }
        return 120 // Default column width
    }
}

#Preview {
    let sampleTable = ToolUIDataTable.Column(id: "model", label: "Model", width: nil)
    
    VStack {
        // Preview would go here
        Text("Data Table Preview")
    }
    .padding()
    .background(Theme.backgroundPrimary)
}
