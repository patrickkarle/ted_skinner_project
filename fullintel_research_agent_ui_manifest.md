manifest:
  id: "PROTO-FULLINTEL-SALES-001"
  version: "1.0.0"
  name: "Fullintel Sales Intelligence Generator"
  description: "Automated sales research, scenario analysis, and outreach generation protocol."

# ------------------------------------------------------------------
# DATA SCHEMAS (The Shape of Data)
# ------------------------------------------------------------------
schemas:
  CompanyProfile:
    fields:
      - name: company_name
      - name: industry_classification
      - name: revenue_tier
      - name: geographic_footprint
      - name: communications_leader_name
      - name: communications_leader_title

  SituationAnalysis:
    fields:
      - name: scenario_type
        enum: [CRISIS, LAUNCH, MA, REGULATORY, COMPETITIVE, EXECUTIVE]
      - name: coverage_volume
      - name: coverage_momentum
      - name: urgency_level
        enum: [HIGH, MEDIUM, LOW]

# ------------------------------------------------------------------
# EXECUTION PHASES (The Workflow)
# ------------------------------------------------------------------
phases:
  - id: "PHASE-01-CONTEXT"
    name: "Context & Firmographics"
    tools: ["search_tool", "finance_api"]
    input: "target_company"
    instructions: |
      Research the target company. Identify:
      1. Revenue/Size
      2. Public/Private status
      3. Specific Industry (e.g., 'SaaS' not just 'Tech')
      4. Recent major events (90 days).
    output_schema: "CompanyProfile"

  - id: "PHASE-02-SITUATION"
    name: "Situation Analysis & Trigger ID"
    tools: ["news_search_tool", "sentiment_analysis"]
    dependencies: ["PHASE-01-CONTEXT"]
    instructions: |
      Analyze news from the last 14 days. Classify into ONE scenario:
      - CRISIS (Recalls, Legal, Breaches)
      - LAUNCH (New products, Campaigns)
      - MA (Mergers, Acquisitions)
      - REGULATORY (FDA, SEC, Labor)
      - COMPETITIVE (Rival moves)
      - EXECUTIVE (C-Suite changes)
      
      Determine coverage momentum (increasing/stable/declining).
    output_schema: "SituationAnalysis"

  - id: "PHASE-03-PAIN-MAPPING"
    name: "Comms Team Intelligence"
    tools: ["linkedin_search_tool"]
    dependencies: ["PHASE-02-SITUATION"]
    instructions: |
      Identify the VP/Director of Comms.
      Map the identified 'scenario_type' to specific pain points:
      - IF Crisis: "Leadership demanding updates, missing social coverage"
      - IF Launch: "Can't prove ROI, key messages not landing"
      - IF MA: "Stakeholder narrative fragmentation"
    output_target: "pain_points_list"

  - id: "PHASE-04-SOLUTION-MATCH"
    name: "Solution & Case Study Matching"
    dependencies: ["PHASE-02-SITUATION"]
    logic_map:
      CRISIS: 
        primary: "24/7 Situation Management"
        case_study: "Florida Gulf Coast University"
      LAUNCH: 
        primary: "Strategic Media Analysis"
        case_study: "Industry Specific or HelloFresh"
      EXECUTIVE: 
        primary: "Executive News Briefings"
        case_study: "Disney"
    instructions: |
      Select Primary Solution based on logic_map.
      Select 1-2 Supporting Solutions.
      Calculate ROI based on scenario (use formula from original prompt).
    output_target: "solution_package"

  - id: "PHASE-05-DRAFTING"
    name: "Brief Generation"
    model: "claude-3-5-sonnet"
    dependencies: ["ALL"]
    instructions: |
      Synthesize all previous outputs into the 'FULLINTEL OPPORTUNITY BRIEF'.
      MUST follow the exact markdown format provided in the system prompt.
      MUST include the 'Initial Outreach Email' and 'Demo Talking Points'.
    output_format: "markdown_file"

# ------------------------------------------------------------------
# QUALITY GATES (The Constraints)
# ------------------------------------------------------------------
quality_gates:
  - phase: "PHASE-02-SITUATION"
    check: "Is coverage_volume quantified?"
    fail_action: "RETRY_SEARCH"
    
  - phase: "PHASE-05-DRAFTING"
    check: "Does output contain 'generic' or 'placeholder' text?"
    fail_action: "REGENERATE_WITH_PENALTY"

  - phase: "PHASE-05-DRAFTING"
    check: "Are ROI calculations present and specific?"
    fail_action: "RECALCULATE_ROI"

  - phase: "PHASE-05-DRAFTING"
    check: "Is a specific, relevant case study included?"
    fail_action: "SEARCH_CASE_STUDIES"