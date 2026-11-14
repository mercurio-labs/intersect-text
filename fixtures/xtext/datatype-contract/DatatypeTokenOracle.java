// Intersect-authored independent oracle. Executes the pinned Xtext implementation.
// No Xtext implementation is copied here; Java is never a production dependency.
import org.eclipse.xtext.parser.impl.DatatypeRuleToken;

public class DatatypeTokenOracle {
    static final class Probe extends DatatypeRuleToken {
        Probe token(String text, int offset) { internalMerge(text, offset); return this; }
        Probe nested(Probe value) { merge(value); return this; }
    }
    static String json(String s) {
        if (s == null) return "null";
        StringBuilder out = new StringBuilder("\"");
        for (char c : s.toCharArray()) {
            if (c == '\\' || c == '\"') out.append('\\').append(c);
            else if (c < 32 || c > 126) out.append(String.format("\\u%04x", (int)c));
            else out.append(c);
        }
        return out.append('\"').toString();
    }
    static void check(String name, Probe p, String expected, int start, int end) {
        if (!java.util.Objects.equals(expected, p.getText()) || start != p.getStartOffset() || end != p.getExpectedOffset())
            throw new AssertionError(name + ": " + p);
        System.out.println("{\"case\":" + json(name) + ",\"value\":" + json(p.getText()) + ",\"start_utf16\":" + p.getStartOffset() + ",\"expected_utf16\":" + p.getExpectedOffset() + "}");
    }
    public static void main(String[] args) {
        check("compact", new Probe().token("P", 0).token("::", 1).token("A", 3), "P::A", 0, 4);
        check("hidden_gaps", new Probe().token("P", 2).token("::", 6).token("A", 11), "P :: A", 2, 12);
        check("raw_quoted_escape", new Probe().token("\"B\\tC\"", 0), "\"B\\tC\"", 0, 6);
        Probe inner = new Probe().token("A", 0).token("::", 8).token("B", 10);
        check("nested_compressed_gap", new Probe().nested(inner).token(".", 11), "A ::B .", 0, 12);
        check("flat_compressed_gap", new Probe().token("A", 0).token("::", 8).token("B", 10).token(".", 11), "A ::B.", 0, 12);
        check("utf16_adjacency", new Probe().token("\"\uD83D\uDE00\"", 0).token("::", 4).token("A", 6), "\"\uD83D\uDE00\"::A", 0, 7);
        check("empty", new Probe(), null, -1, -1);
    }
}
