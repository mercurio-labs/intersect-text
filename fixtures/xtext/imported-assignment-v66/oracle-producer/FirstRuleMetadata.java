// Independent verification adapter: reports pinned Xtext's resolved object identities
// and GrammarUtil classification. It does not replace or duplicate the validator.
package intersect.oracle;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.xtext.*;
import org.eclipse.xtext.resource.*;
public final class FirstRuleMetadata {
 static Map<String,Object> fields(Object... p){return TerminalNegationOracle.fields(p);}
 public static void main(String[] args)throws Exception {
  Path root=Path.of(args[0]).toAbsolutePath();
  var injector=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();
  var set=injector.getInstance(XtextResourceSet.class);
  var imported=set.getResource(URI.createFileURI(root.resolve("imported.ecore").toString()),true);
  var pkg=(EPackage)imported.getContents().get(0);set.getPackageRegistry().put(pkg.getNsURI(),pkg);
  Map<String,XtextResource> loaded=new LinkedHashMap<>();
  for(String name:Files.readAllLines(root.resolve("order.txt"),StandardCharsets.UTF_8)){
   var resource=(XtextResource)set.createResource(URI.createURI("memory:/"+name.replace('.','/')+".xtext"));
   resource.load(new ByteArrayInputStream(Files.readAllBytes(root.resolve(name+".xtext"))),null);loaded.put(name,resource);
  }
  Object failure=null;try{EcoreUtil.resolveAll(set);}catch(Throwable t){failure=TerminalNegationOracle.failure(t);}
  List<Object> grammars=new ArrayList<>();
  for(var item:loaded.entrySet()){
   if(item.getValue().getContents().isEmpty()){grammars.add(fields("source",item.getKey(),"grammar",null));continue;}
   var grammar=(Grammar)item.getValue().getContents().get(0);List<Object> used=new ArrayList<>(),rules=new ArrayList<>();
   for(var g:grammar.getUsedGrammars())used.add(fields("name",g.getName(),"proxy",g.eIsProxy(),"uri",EcoreUtil.getURI(g).toString()));
   for(var rule:grammar.getRules()){
    var type=rule.getType();var classifier=type==null?null:type.getClassifier();
    Object identity=classifier==null?null:fields("name",classifier.getName(),"kind",classifier.eClass().getName(),"proxy",classifier.eIsProxy(),"namespace",classifier.getEPackage()==null?null:classifier.getEPackage().getNsURI(),"resource_fragment",EcoreUtil.getURI(classifier).fragment());
    rules.add(fields("name",rule.getName(),"kind",rule.eClass().getName(),"fragment",rule instanceof ParserRule?((ParserRule)rule).isFragment():rule instanceof TerminalRule?((TerminalRule)rule).isFragment():null,"datatype",rule instanceof ParserRule?GrammarUtil.isDatatypeRule((ParserRule)rule):null,"classifier",identity));
   }
   grammars.add(fields("source",item.getKey(),"used",used,"rules",rules));
  }
  System.out.println(TerminalNegationOracle.json(fields("grammars",grammars,"linking_failure",failure)));
 }
}
