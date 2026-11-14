// Verification-only recursive view of the pinned upstream parser AST.
package intersect.oracle;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.io.StringReader;
import java.util.*;
import org.eclipse.xtext.XtextStandaloneSetup;
import org.eclipse.xtext.parser.IParser;
import org.eclipse.xtext.conversion.IValueConverterService;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.emf.ecore.*;
public final class TreeOracle {
 static IValueConverterService converters;
 static String object(Map<String,String> fields){StringJoiner j=new StringJoiner(",","{","}");fields.forEach((k,v)->j.add(SyntaxOracle.quote(k)+":"+v));return j.toString();}
 static String list(List<String> values){return "["+String.join(",",values)+"]";}
 static String node(EObject value) {
  Map<String,String> attrs=new TreeMap<>(),children=new TreeMap<>(),refs=new TreeMap<>();
  for(var f:value.eClass().getEAllAttributes())attrs.put(f.getName(),SyntaxOracle.attribute(value.eGet(f)));
  for(var f:value.eClass().getEAllReferences()) {
   List<String> values=new ArrayList<>();
   if(f.isContainment()) {
    Object child=value.eGet(f,false);
    if(child instanceof EObject)values.add(node((EObject)child));
    else if(child instanceof Iterable<?>)for(Object item:(Iterable<?>)child)values.add(node((EObject)item));
    if(!values.isEmpty())children.put(f.getName(),list(values));
   } else {
    String rule=switch(f.getEReferenceType().getName()) {
     case "Grammar"->"GrammarID";
     case "AbstractRule"->"RuleID";
     case "EPackage"->"STRING";
     case "EEnumLiteral"->"ValidID";
     default->"ID";
    };
    for(var n:NodeModelUtils.findNodesForFeature(value,f))values.add(SyntaxOracle.attribute(converters.toValue(NodeModelUtils.getTokenText(n),rule,n)));
    if(!values.isEmpty())refs.put(f.getName(),list(values));
   }
  }
  Map<String,String> out=new TreeMap<>();out.put("class",SyntaxOracle.quote(value.eClass().getName()));out.put("attributes",object(attrs));out.put("children",object(children));out.put("references",object(refs));return object(out);
 }
 public static void main(String[] args) throws Exception {
  var injector=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();
  IParser parser=injector.getInstance(IParser.class);converters=injector.getInstance(IValueConverterService.class);
  for(String arg:args){Path p=Path.of(arg);var result=parser.parse(new StringReader(Files.readString(p,StandardCharsets.UTF_8)));
   System.out.println("{\"id\":"+SyntaxOracle.quote(p.getFileName().toString().replace(".xtext",""))+",\"syntax_errors\":"+result.hasSyntaxErrors()+",\"tree\":"+(result.hasSyntaxErrors()?"null":node(result.getRootASTElement()))+"}");
  }
 }
}
