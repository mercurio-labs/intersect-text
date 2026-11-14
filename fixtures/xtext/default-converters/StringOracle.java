// Intersect-authored optional verification adapter; no production Java.
package intersect.oracle;
import org.eclipse.xtext.conversion.impl.STRINGValueConverter;
public final class StringOracle {
 public static void main(String[] args) throws Exception {
  var converter = new STRINGValueConverter();
  for (String line : java.nio.file.Files.readAllLines(java.nio.file.Path.of(args[0]), java.nio.charset.StandardCharsets.UTF_8)) {
   StringBuilder input = new StringBuilder();
   for (String unit : line.split(" ")) input.append((char)Integer.parseInt(unit, 16));
   String literal = input.toString();
   String prefix = "{\"literal\":" + SyntaxOracle.quote(literal);
   try {
    String value = converter.toValue(literal, null);
    System.out.println(prefix + ",\"value\":" + SyntaxOracle.attribute(value) + "}");
   } catch (RuntimeException error) {
    System.out.println(prefix + ",\"error\":" + SyntaxOracle.quote(error.getClass().getSimpleName()) + ",\"message\":" + SyntaxOracle.quote(error.getMessage()) + "}");
   }
  }
 }
}
