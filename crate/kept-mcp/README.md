# kept-mcp

Stdio Model Context Protocol (MCP) Server (`bl1nk-kept-mcp`) เชื่อมต่อ AI Agent กับ Workspace

---

## หน้าที่หลัก
1. **Context Admission Gateway:** ใช้ Judge Engine ตรวจสอบ context ก่อนส่งให้ AI Agent
2. **MCP Tools:** ให้บริการเครื่องมือ Workspace Filesystem, Search, Document และ Memory
3. **Session State Tracking:** จัดเก็บ Context Registry ร่วมกับ SQLite backend
