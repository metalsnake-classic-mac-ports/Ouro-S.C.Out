// Ghidra headless post-script: create functions at every LINK A6 (0x4E56)
// on an even address and at the offsets listed in a seed file (one hex
// offset per line, e.g. from the jump table), then re-run auto analysis.
// Argument (optional): seed file path.
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.mem.*;
import java.nio.file.*;

public class CreateFuncs extends GhidraScript {
    @Override
    public void run() throws Exception {
        Memory mem = currentProgram.getMemory();
        Address base = currentProgram.getMinAddress();
        int created = 0;
        String[] args = getScriptArgs();
        if (args.length > 0 && Files.exists(Paths.get(args[0]))) {
            for (String line : Files.readAllLines(Paths.get(args[0]))) {
                line = line.trim();
                if (line.isEmpty()) continue;
                Address a = base.add(Long.parseLong(line, 16));
                disassemble(a);
                if (getFunctionAt(a) == null && createFunction(a, null) != null) created++;
            }
        }
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
