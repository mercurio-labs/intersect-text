// Intersect-authored independent oracle. No production Java dependency.
import org.eclipse.emf.ecore.EcorePackage;
import org.eclipse.xtext.conversion.impl.EFactoryValueConverter;
public class DoubleOracle {
  static String json(String text) {
    StringBuilder out = new StringBuilder("\"");
    for (char c : text.toCharArray()) {
      if (c == '\\' || c == '"') out.append('\\').append(c);
      else if (c < 32 || c > 126) out.append(String.format("\\u%04x", (int)c));
      else out.append(c);
    }
    return out.append('"').toString();
  }
  public static void main(String[] args) {
    var converter = new EFactoryValueConverter(EcorePackage.Literals.EDOUBLE);
    for (String text : args) {
      try {
        double value = (Double)converter.toValue(text, null);
        System.out.println("{\"text\":" + json(text) + ",\"accepted\":true,\"bits\":\"" + String.format("%016x", Double.doubleToLongBits(value)) + "\"}");
      } catch (org.eclipse.xtext.conversion.ValueConverterException error) {
        System.out.println("{\"text\":" + json(text) + ",\"accepted\":false}");
      }
    }
  }
}
