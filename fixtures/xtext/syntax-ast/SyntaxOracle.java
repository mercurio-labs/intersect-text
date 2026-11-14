// Independent Xtext 2.38 parser adapter, verification only. No production dependency.
package intersect.oracle;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.io.StringReader;
import org.eclipse.xtext.XtextStandaloneSetup;
import org.eclipse.xtext.parser.IParser;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.emf.ecore.EObject;
public final class SyntaxOracle {
 static String quote(String text) {
  StringBuilder b=new StringBuilder("\"");
  for(char c:text.toCharArray()) {
   if(c=='"'||c=='\\') b.append('\\').append(c);
   else if(c<32||c>126) b.append(String.format("\\u%04x",(int)c));
   else b.append(c);
  }
  return b.append('"').toString();
 }
 static String attribute(Object value) {
  if(value==null)return "null";
  if(!(value instanceof String))return quote(value.toString());
  StringBuilder b=new StringBuilder("{\"utf16\":[");boolean first=true;
  for(char c:((String)value).toCharArray()){if(!first)b.append(',');first=false;b.append((int)c);}
  return b.append("]}").toString();
 }
 static String node(EObject object,String source) {
  var n=NodeModelUtils.getNode(object);
  StringBuilder b=new StringBuilder("{\"class\":").append(quote(object.eClass().getName()));
  if(n!=null) b.append(",\"start\":").append(source.substring(0,n.getOffset()).getBytes(StandardCharsets.UTF_8).length).append(",\"end\":").append(source.substring(0,n.getEndOffset()).getBytes(StandardCharsets.UTF_8).length);
  b.append(",\"attributes\":{"); boolean first=true;
  for(var f:object.eClass().getEAllAttributes()) {
   if(!first)b.append(',');first=false;
   var value=object.eGet(f);
   b.append(quote(f.getName())).append(':').append(attribute(value));
  }
  return b.append("}}").toString();
 }
 public static void main(String[] args) throws Exception {
  IParser parser=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration().getInstance(IParser.class);
  for(String arg:args) {
   Path path=Path.of(arg);String source=Files.readString(path,StandardCharsets.UTF_8);
   var result=parser.parse(new StringReader(source));
   StringBuilder out=new StringBuilder("{\"id\":").append(quote(path.getFileName().toString().replace(".xtext","")));
   out.append(",\"syntax_errors\":").append(result.hasSyntaxErrors()).append(",\"nodes\":[");
   EObject root=result.getRootASTElement();
   if(root!=null) {
    out.append(node(root,source));
    for(var it=root.eAllContents();it.hasNext();)out.append(',').append(node(it.next(),source));
   }
   out.append("],\"errors\":[");boolean first=true;
   for(var error:result.getSyntaxErrors()) {if(!first)out.append(',');first=false;out.append(quote(error.getSyntaxErrorMessage().getMessage()));}
   System.out.println(out.append("]}"));
  }
 }
}
