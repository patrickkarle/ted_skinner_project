# L0: Stakeholder Requirements Document

**Document ID:** L0-REQ-FULLINTEL-001
**Project:** Fullintel Sales Intelligence Generator
**Stakeholder:** Ted Skinner (Fullintel)
**Date:** 2025-11-19
**Version:** 1.0

---

## 1. Stakeholder Identity

**Organization:** Fullintel
**Primary Stakeholder:** Ted Skinner
**End Users:** Fullintel sales team
**Business Domain:** Sales intelligence and outreach automation

---

## 2. Problem Statement

### 2.1 Current State
Fullintel sales team currently spends **2-4 hours per prospect** conducting manual research:
- Company background and firmographics research
- Recent news and situation analysis
- Communications team identification
- Pain point mapping
- Solution matching
- Outreach brief writing

### 2.2 Pain Points
- **Time-consuming:** Manual research reduces number of prospects engaged
- **Inconsistent quality:** Brief quality varies by salesperson
- **Delayed response:** Opportunities missed due to slow research cycle
- **High cognitive load:** Repetitive research tasks cause fatigue
- **Limited scalability:** Cannot scale sales team without proportional cost increase

### 2.3 Business Impact
- Reduced sales pipeline velocity
- Missed time-sensitive opportunities
- Inconsistent customer experience
- Higher sales team overhead costs

---

## 3. Stakeholder Requirements

### SR-001: Research Time Reduction
**Priority:** CRITICAL
**Statement:** "I need to reduce research time from 2-4 hours to under 5 minutes per prospect"
**Success Metric:** Average research time < 5 minutes
**Rationale:** Time savings directly increases pipeline capacity

### SR-002: Quality Standardization
**Priority:** CRITICAL
**Statement:** "All sales briefs must meet minimum quality standards - no generic text or placeholders"
**Success Metric:**
- Zero generic/placeholder text in final output
- ROI calculations present in 100% of briefs
- Specific case studies included in 100% of briefs
**Rationale:** Consistent quality improves close rates

### SR-003: Cost Control
**Priority:** HIGH
**Statement:** "The cost per research brief must be under $0.10 to justify ROI"
**Success Metric:** Average cost per brief < $0.10 (LLM + tools)
**Rationale:** Must be economically viable at scale

### SR-004: Offline Capability
**Priority:** MEDIUM
**Statement:** "Sales team should be able to work with cached data when offline"
**Success Metric:** App functions with cached company data (no live APIs required)
**Rationale:** Sales reps travel frequently, need offline access

### SR-005: Easy Export
**Priority:** HIGH
**Statement:** "I need one-click export to PDF and clipboard for quick sharing"
**Success Metric:**
- Copy to clipboard: < 1 second
- Export to PDF: < 3 seconds
**Rationale:** Integration with existing sales workflow (email, CRM)

### SR-006: Desktop Application
**Priority:** CRITICAL
**Statement:** "Must be a desktop app, not web-based - data stays local"
**Success Metric:**
- Runs on Windows 10+, macOS 11+
- No cloud dependencies for core functionality
- Data stored locally only
**Rationale:** Data security, privacy compliance, reliability

### SR-007: API Key Security
**Priority:** HIGH
**Statement:** "API keys must be stored securely, not in plain text"
**Success Metric:** API keys encrypted using OS credential manager
**Rationale:** Security compliance, prevent unauthorized usage

### SR-008: Session History
**Priority:** MEDIUM
**Statement:** "I want to see past research briefs and re-export them"
**Success Metric:**
- Access to all research history
- Search by company name
- Re-export previous briefs
**Rationale:** Reference past work, track engagement history

### SR-009: Error Recovery
**Priority:** HIGH
**Statement:** "If the app crashes mid-research, I should be able to resume where I left off"
**Success Metric:** Auto-resume from last completed phase
**Rationale:** Prevents wasted LLM costs, better UX

### SR-010: Progress Visibility
**Priority:** MEDIUM
**Statement:** "I want to see real-time progress during the 5-minute research process"
**Success Metric:**
- Live phase indicators (1/5, 2/5, etc.)
- Real-time log output
- Estimated time remaining
**Rationale:** User confidence, reduces perceived wait time

---

## 4. Workflow Requirements

### WF-001: Five-Phase Research Process
**Requirement:** System must execute exactly 5 phases in sequence:

1. **Phase 1: Context & Firmographics**
   - Input: Company name
   - Output: CompanyProfile (industry, revenue tier, geographic footprint)
   - Tools: Web search, company database APIs

2. **Phase 2: Situation Analysis**
   - Input: CompanyProfile
   - Output: SituationAnalysis (scenario type, coverage volume, urgency)
   - Tools: News search (14-day lookback), sentiment analysis

3. **Phase 3: Communications Team Intelligence**
   - Input: SituationAnalysis
   - Output: Pain points list, contact information
   - Tools: LinkedIn search, manual input fallback

4. **Phase 4: Solution Matching**
   - Input: SituationAnalysis
   - Output: Fullintel solution package, case study
   - Logic: Rule-based mapping (scenario → solution)

5. **Phase 5: Brief Generation**
   - Input: ALL previous phase outputs
   - Output: Markdown brief (FULLINTEL OPPORTUNITY BRIEF format)
   - Model: Claude 3.5 Sonnet

### WF-002: Quality Gates
**Requirement:** System must enforce quality gates:
- **Phase 2:** Coverage volume must be quantified (not "significant coverage")
- **Phase 5:** No generic/placeholder text allowed
- **Phase 5:** ROI calculations must be specific and present
- **Phase 5:** Case study must be specific and relevant

**Enforcement:** Block completion if quality gates fail, offer regeneration

---

## 5. Data Requirements

### DR-001: Company Data
**Required Fields:**
- Company name (required)
- Industry classification (required)
- Revenue tier (required)
- Geographic footprint (required)
- Communications leader name (optional)
- Communications leader title (optional)

### DR-002: News Data
**Required Fields:**
- Article headlines (minimum 5)
- Publication dates (last 14 days)
- Coverage momentum trend (increasing/stable/declining)
- Total article count (quantified number)

### DR-003: Case Study Data
**Required Fields:**
- Client name
- Scenario type match
- Results achieved (quantified)
- Timeframe

### DR-004: Output Format
**Required Structure:**
```markdown
# FULLINTEL OPPORTUNITY BRIEF
## Target Company: [Company Name]

### Executive Summary
[2-3 sentence overview]

### Situation Analysis
- Scenario Type: [CRISIS/LAUNCH/MA/REGULATORY/COMPETITIVE/EXECUTIVE]
- Coverage Volume: [Quantified number] articles in last 14 days
- Momentum: [Increasing/Stable/Declining]
- Urgency Level: [HIGH/MEDIUM/LOW]

### Pain Points
[Bulleted list of specific pain points]

### Recommended Solution
[Specific Fullintel solution package]

### ROI Projection
[Quantified projections with assumptions]

### Relevant Case Study
[Specific client example with results]

### Next Steps
[Actionable outreach recommendations]
```

---

## 6. Technical Constraints

### TC-001: Technology Stack
**Constraint:** Must use Tauri + Rust + React
**Rationale:** Already in development, team expertise, cross-platform desktop

### TC-002: LLM Providers
**Constraint:** Support multiple LLM providers with fallback
**Approved Providers:**
- Anthropic Claude (primary for Phase 5)
- Google Gemini (fallback)
- DeepSeek (cost optimization for Phases 1-3)

### TC-003: Deployment
**Constraint:** Desktop application only (no web version in MVP)
**Platforms:** Windows 10+, macOS 11+, Linux (Ubuntu 20.04+)

### TC-004: Data Retention
**Constraint:** Auto-delete research sessions older than 90 days
**Rationale:** Privacy compliance (GDPR/CCPA)

---

## 7. Non-Functional Requirements

### NFR-001: Performance
- **Research completion time:** < 5 minutes end-to-end
- **Phase execution time:** < 60 seconds per phase average
- **UI responsiveness:** < 100ms for user interactions
- **Application startup:** < 3 seconds

### NFR-002: Reliability
- **Uptime:** 99%+ (local application)
- **Error recovery:** Auto-retry 3x before failure
- **Crash recovery:** Resume from last completed phase
- **Data persistence:** Zero data loss on crash

### NFR-003: Usability
- **Setup time:** < 2 minutes (first-time configuration)
- **Learning curve:** < 10 minutes to proficiency
- **Error messages:** Actionable and specific
- **Help documentation:** Context-sensitive

### NFR-004: Security
- **API keys:** Encrypted storage (OS credential manager)
- **Input sanitization:** Prevent injection attacks
- **Rate limiting:** Max 10 LLM calls/minute
- **Audit logging:** All research sessions logged

### NFR-005: Maintainability
- **Code coverage:** > 80%
- **Documentation:** All public APIs documented
- **Modularity:** Easy to swap tools/LLMs
- **Logging:** Debug mode available

### NFR-006: Cost
- **Cost per brief:** < $0.10 total
- **Budget monitoring:** Real-time cost tracking
- **Cost alerts:** Warn user at $10/month threshold

---

## 8. Success Criteria

### Business Metrics
| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Research time reduction | 95%+ | Before: 2-4 hrs → After: < 5 min |
| Briefs generated per day | 10x increase | Quantity per sales rep |
| Brief quality consistency | 90%+ pass rate | Quality gate pass percentage |
| Sales team adoption | 80%+ active use | Daily active users |
| Cost per brief | < $0.10 | API costs + tool costs |

### Technical Metrics
| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| Application uptime | 99%+ | Crash reports |
| Quality gate pass rate | 90%+ | First-time pass percentage |
| Error recovery success | 95%+ | Resume after crash |
| User satisfaction | > 4/5 | Post-use survey |

---

## 9. Out of Scope (MVP)

### Explicitly NOT Included
- ❌ Web version
- ❌ Team collaboration features
- ❌ CRM integration (Salesforce/HubSpot)
- ❌ Email sending integration
- ❌ Multi-user permissions
- ❌ Cloud sync
- ❌ Mobile applications
- ❌ Real-time collaboration
- ❌ Analytics dashboard
- ❌ A/B testing different prompts

### Future Enhancements (Post-MVP)
- Integration with Salesforce/HubSpot
- Email template generation
- Team sharing and commenting
- Analytics and ROI tracking
- Custom prompt templates
- Automated LinkedIn outreach

---

## 10. Acceptance Criteria

### Minimum Viable Product (MVP) Acceptance
The system will be considered ACCEPTED when:

1. ✅ User can input company name and generate brief in < 5 minutes
2. ✅ Quality gates enforce "no generic text" rule successfully
3. ✅ Cost per brief consistently < $0.10
4. ✅ Application runs offline with cached data
5. ✅ One-click export to PDF and clipboard works reliably
6. ✅ Session recovery works after application crash
7. ✅ API keys stored securely (not plain text)
8. ✅ 90-day auto-delete of old sessions implemented
9. ✅ Real-time progress indication during research
10. ✅ All quality gates pass for 9 out of 10 test cases

### Testing Requirements
- **Unit tests:** > 80% code coverage
- **Integration tests:** All 5 phases tested end-to-end
- **User acceptance testing:** 5 sales reps test with real prospects
- **Performance testing:** 20 consecutive briefs < 5 min each

---

## 11. Timeline Expectations

**Stakeholder Expectation:** Working prototype TODAY (within hours, not weeks)

**Breakdown:**
- Requirements → Architecture: COMPLETE
- Detailed design: 30 minutes
- Implementation: 3-4 hours
- Testing: 1 hour
- Total: ~6-7 hours to working MVP

---

## 12. Open Questions for Stakeholder

1. **Tool Access:** Does Fullintel have existing API access to company databases (Crunchbase, PitchBook) or news services?
2. **Budget:** What's the monthly budget for API costs (LLM + tools)?
3. **LinkedIn Strategy:** Are we allowed to use LinkedIn APIs, or should we use alternative contact databases?
4. **Data Retention:** Is 90-day auto-delete acceptable, or different retention period needed?
5. **Export Formats:** PDF + clipboard sufficient, or need Word format too?
6. **CRM Integration:** Which CRM does sales team use (for future integration)?

---

**Document Status:** Complete
**Traceability:** This L0 document flows down to L1-SAD (System Architecture)
**Next Document:** L1-SAD-1.1-MissionIntent.md
