// Ghidra headless post-script: create a function at every LINK A6 (0x4E56)
// on an even address, then re-run auto analysis.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.mem.*;

public class CreateFuncs extends GhidraScript {
    @Override
    public void run() throws Exception {
        Memory mem = currentProgram.getMemory();
        int created = 0;
        for (MemoryBlock b : mem.getBlocks()) {
            Address a = b.getStart();
            while (a.compareTo(b.getEnd()) < 0) {
                if ((mem.getShort(a) & 0xFFFF) == 0x4E56) {
                    disassemble(a);
                    if (getFunctionAt(a) == null && createFunction(a, null) != null) created++;
                }
                a = a.add(2);
            }
        }
        analyzeAll(currentProgram);
        println("created functions: " + created);
    }
}
