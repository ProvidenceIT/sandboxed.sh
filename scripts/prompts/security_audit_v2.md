# Security Audit Task

## PHASE 0: MANDATORY WORKSPACE SETUP (DO THIS FIRST)

Before ANY analysis, you MUST complete these steps:

### Step 1: Create your isolated workspace
```bash
mkdir -p /root/work/security-audit-{your-model-name}/{source,output,temp,notes}
```

### Step 2: Acquire the source code INTO your workspace
**Clone directly into YOUR workspace** (do NOT use /root/context/):
```bash
cd /root/work/security-audit-{your-model-name}/source
git clone https://github.com/RabbyHub/Rabby .
```

If git fails, download the CRX:
```bash
curl -L "https://clients2.google.com/service/update2/crx?response=redirect&x=id%3Dacmacodkjbdgmoleebolmdjonilkdbch%26uc" \
  -o /root/work/security-audit-{your-model-name}/temp/rabby.crx
unzip /root/work/security-audit-{your-model-name}/temp/rabby.crx -d /root/work/security-audit-{your-model-name}/source/
```

### Step 3: Verify your sources exist
```bash
ls -la /root/work/security-audit-{your-model-name}/source/
# You should see Rabby wallet files (package.json, src/, _raw/, etc.)
```

### Step 4: Create source manifest
Write a `notes/sources.md` documenting:
- Where the sources came from (GitHub/CRX)
- Total file count
- Key directories identified

⚠️ **DO NOT PROCEED** until your `/root/work/security-audit-{model}/source/` folder has Rabby files.

---

## TARGET
**Rabby Wallet Chrome Extension** - A cryptocurrency wallet with transaction simulation.

GitHub: https://github.com/RabbyHub/Rabby

## SCOPE - FOCUS ONLY ON THESE AREAS
1. **Transaction Simulation Bypass** - Can attackers make harmful transactions appear safe?
2. **Approval Amount Manipulation** - Can displayed approval amounts differ from actual?
3. **Spender Address Spoofing** - Can fake addresses be shown as trusted protocols?
4. **Permit2 Integration** - Validation of spender field against known reactors/protocols

## REFERENCE VULNERABILITY (Example of what to find)
A previous critical bug was found where Permit2 transactions could bypass simulation:
- **Symptom**: Simulation showed "Spend 1 USDC to receive 1337 ETH"
- **Reality**: Transaction approved 100,000 USDC to attacker's vanity address
- **Root cause**: The `spender` field in Permit2 was not validated against trusted addresses
- **Why it worked**: Rabby trusted the `witness` data for simulation, but the witness can only be trusted if the spender is a known protocol (like Uniswap's reactor)
- **Impact**: Full balance drain of any approved token

Your goal is to find similar issues where trust assumptions allow bypassing security checks.

## KEY FILES TO ANALYZE (in YOUR source folder)
Search within `/root/work/security-audit-{model}/source/` for:
- `src/background/` - Main extension logic
- Files containing: `Permit2`, `signTypedData`, `eth_sendTransaction`, `securityEngine`
- `_raw/` - Built extension assets
- Transaction preview/simulation components
- Approval handling and display logic

## ANALYSIS RULES

⛔ **FORBIDDEN - DO NOT DO THESE:**
- Do NOT read or analyze `/root/context/*` (may contain unrelated files)
- Do NOT analyze `.jar` files, Minecraft plugins, or non-Rabby code
- Do NOT create files outside your `/root/work/security-audit-{model}/` folder
- Do NOT stop without producing the full AUDIT_REPORT.md

✅ **REQUIRED:**
- ONLY analyze files in `/root/work/security-audit-{model}/source/`
- Index files using `index_files` on your source folder
- Use `search_file_index` and `grep_search` on your source folder
- Document ALL findings in `/root/work/security-audit-{model}/output/AUDIT_REPORT.md`

## METHODOLOGY

1. **Setup Phase** (subtasks 1-2):
   - Create workspace structure
   - Clone Rabby source into your workspace
   - Verify sources, create manifest

2. **Discovery Phase** (subtasks 3-4):
   - Index all files in source/
   - Search for Permit2, approval, simulation keywords
   - Map key files and their purposes

3. **Analysis Phase** (subtasks 5-8):
   - Deep-dive into Permit2 handling
   - Trace data flow: user input → simulation → display
   - Identify trust boundaries
   - Find validation gaps

4. **Documentation Phase** (subtasks 9-10):
   - Document each finding with full details
   - Write AUDIT_REPORT.md
   - Call complete_mission with report content

## DELIVERABLE (REQUIRED)

Your FINAL message MUST contain the complete `AUDIT_REPORT.md` in markdown format.

```markdown
# Rabby Wallet Security Audit Report

**Auditor**: [your model name]
**Date**: [today's date]
**Source**: GitHub RabbyHub/Rabby (commit: [hash])
**Scope**: Transaction simulation, Permit2, Approval handling

## Executive Summary
[2-3 sentences on overall security posture]

## Critical Findings

### [SEVERITY] Finding Title
- **Location**: `src/path/to/file.ts:123`
- **Description**: Technical explanation
- **Attack Scenario**: How an attacker exploits this
- **Impact**: Token theft / Approval hijack / etc.
- **PoC Concept**: Steps to reproduce
- **Recommendation**: How to fix

## Medium/Low Findings
[Same format]

## Code Quality Observations
[Patterns, missing validations]

## Files Analyzed
| File | Purpose | Findings |
|------|---------|----------|
| src/background/... | ... | ... |

## Conclusion
[Summary and recommendations]
```

## SUCCESS CRITERIA

1. ✅ Source code cloned to YOUR workspace (not /root/context/)
2. ✅ Analysis focused ONLY on Rabby Wallet code
3. ✅ At least 3 potential findings documented
4. ✅ AUDIT_REPORT.md produced with full template
5. ✅ Report included in final message (not just file path)
