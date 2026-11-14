// Verification-only source-loaded hidden-rule identity observations.
package intersect.oracle;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.xtext.*;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.resource.*;
import org.eclipse.xtext.util.CancelIndicator;
import org.eclipse.xtext.validation.*;

public final class HiddenScopeMetadata {
 static Map<String,Object> fields(Object... v){return TerminalNegationOracle.fields(v);}
 static String sha(byte[] bytes)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));}
 static Object node(EObject object){if(object==null)return null;var n=NodeModelUtils.getNode(object);return n==null?null:fields("utf16_start",n.getOffset(),"utf16_end",n.getEndOffset(),"text",NodeModelUtils.getTokenText(n));}
 static Object failure(Throwable t){return fields("class",t.getClass().getName(),"message",t.getMessage());}
 static String owner(AbstractRule r){if(r==null||r.eIsProxy())return null;var g=GrammarUtil.getGrammar(r);return g==null?null:g.getName()+"::"+r.getName();}
 static Object feature(EObject object,String name){var f=object.eClass().getEStructuralFeature(name);if(f==null)throw new IllegalArgumentException("Missing observed Xtext feature "+name);return object.eGet(f);}
 static Object rule(AbstractRule r,Map<String,Object> ids){if(r==null)return null;var g=r.eIsProxy()?null:GrammarUtil.getGrammar(r);return fields("identity",owner(r),"proxy",r.eIsProxy(),"kind",r.eClass().getName(),"name",r.getName(),"fragment",r instanceof ParserRule?Boolean.valueOf(((ParserRule)r).isFragment()):r instanceof TerminalRule?Boolean.valueOf(((TerminalRule)r).isFragment()):null,"source",g==null?null:ids.get(g.getName()),"resource_uri",r.eResource()==null?null:r.eResource().getURI().toString(),"node",node(r));}
 static Object hidden(EObject object,Map<String,Object> ids){List<Object> values=new ArrayList<>();for(Object value:(Iterable<?>)feature(object,"hiddenTokens"))values.add(rule((AbstractRule)value,ids));return fields("defines_hidden_tokens",feature(object,"definesHiddenTokens"),"resolved_hidden_tokens",values);}
 public static void main(String[] args)throws Exception{
  if(args.length!=1)throw new IllegalArgumentException("Expected case root containing frozen bindings");
  Path root=Path.of(args[0]).toAbsolutePath().normalize();
  var injector=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();var set=injector.getInstance(XtextResourceSet.class);
  Set<String> uris=new HashSet<>(),files=new HashSet<>();Map<String,String> logical=new LinkedHashMap<>();Map<String,Object> ids=new LinkedHashMap<>();String provider=null;
  for(String line:Files.readAllLines(root.resolve("uri-bindings.tsv"),StandardCharsets.UTF_8)){
   String[] p=line.split("\t",-1);if(p.length!=5||!p[3].matches("[0-9a-f]{64}")||!uris.add(p[2])||!files.add(p[4]))throw new IllegalArgumentException("Invalid or ambiguous URI binding");
   Path physical=root.resolve(p[4]).normalize();if(!physical.getParent().equals(root)||!sha(Files.readAllBytes(physical)).equals(p[3]))throw new IllegalArgumentException("Escaping or mismatched URI binding");
   URI uri=URI.createURI(p[2]);if(uri.isRelative()||uri.hasFragment()||uri.hasQuery())throw new IllegalArgumentException("Logical URI must be absolute without fragment/query");
   if(set.getURIConverter().getURIMap().put(uri,URI.createFileURI(physical.toString()))!=null)throw new IllegalArgumentException("Duplicate URI mapping");
   if(p[0].equals("provider")){if(provider!=null)throw new IllegalArgumentException("Multiple providers");provider=p[2];}
   else if(p[0].equals("grammar")){if(logical.put(p[1],p[2])!=null)throw new IllegalArgumentException("Duplicate grammar");ids.put(p[1],fields("uri",p[2],"sha256",p[3]));}
   else throw new IllegalArgumentException("Unknown binding kind");
  }
  if(provider==null)throw new IllegalArgumentException("Missing provider");
  var imported=set.getResource(URI.createURI(provider),true);var pkg=(EPackage)imported.getContents().get(0);set.getPackageRegistry().put(pkg.getNsURI(),pkg);
  Map<String,XtextResource> loaded=new LinkedHashMap<>();Map<String,Grammar> grammars=new LinkedHashMap<>();List<Object> syntax=new ArrayList<>();
  for(String name:Files.readAllLines(root.resolve("order.txt"),StandardCharsets.UTF_8)){
   if(!logical.containsKey(name))throw new IllegalArgumentException("Unbound ordered grammar");
   var resource=(XtextResource)set.getResource(URI.createURI(logical.get(name)),true);loaded.put(name,resource);grammars.put(name,(Grammar)resource.getContents().get(0));
   List<Object> errors=new ArrayList<>();for(var n:resource.getParseResult().getSyntaxErrors())errors.add(fields("utf16_start",n.getOffset(),"utf16_end",n.getEndOffset(),"message",n.getSyntaxErrorMessage().getMessage()));syntax.add(fields("grammar",name,"source",ids.get(name),"errors",errors));
  }
  Object resolveFailure=null;try{EcoreUtil.resolveAll(set);}catch(Throwable t){resolveFailure=failure(t);}
  List<Object> observations=new ArrayList<>();var validator=injector.getInstance(IResourceValidator.class);
  for(var item:grammars.entrySet()){
   Grammar g=item.getValue();List<Object> declared=new ArrayList<>(),used=new ArrayList<>(),errors=new ArrayList<>(),warnings=new ArrayList<>(),issues=new ArrayList<>();
   for(var parent:g.getUsedGrammars())used.add(fields("name",parent.getName(),"proxy",parent.eIsProxy(),"source",ids.get(parent.getName()),"resource_uri",parent.eResource()==null?null:parent.eResource().getURI().toString()));
   for(var r:g.getRules())declared.add(fields("rule",rule(r,ids),"authored_hidden",r instanceof ParserRule?hidden(r,ids):null));
   for(var d:loaded.get(item.getKey()).getErrors())errors.add(fields("message",d.getMessage(),"line",d.getLine(),"column",d.getColumn()));for(var d:loaded.get(item.getKey()).getWarnings())warnings.add(fields("message",d.getMessage(),"line",d.getLine(),"column",d.getColumn()));
   Object validationFailure=null;try{for(var issue:validator.validate(loaded.get(item.getKey()),CheckMode.ALL,CancelIndicator.NullImpl))issues.add(fields("severity",issue.getSeverity().name(),"code",issue.getCode(),"utf16_start",issue.getOffset(),"length",issue.getLength(),"message",issue.getMessage(),"uri_to_problem",issue.getUriToProblem()==null?null:issue.getUriToProblem().toString()));}catch(Throwable t){validationFailure=failure(t);}issues.sort(Comparator.comparing(TerminalNegationOracle::json));
   observations.add(fields("grammar",g.getName(),"source",ids.get(g.getName()),"resource_uri",g.eResource().getURI().toString(),"grammar_hidden",hidden(g,ids),"used_grammars",used,"declared_rules",declared,"errors",errors,"warnings",warnings,"validation",issues,"validation_failure",validationFailure));
  }
  String entry=Files.readString(root.resolve("entry.txt"),StandardCharsets.UTF_8).trim();List<Object> effective=new ArrayList<>();Object effectiveFailure=null;try{for(var r:GrammarUtil.allRules(grammars.get(entry)))effective.add(rule(r,ids));}catch(Throwable t){effectiveFailure=failure(t);}
  System.out.println(TerminalNegationOracle.json(fields("entry",entry,"syntax",syntax,"resolve_failure",resolveFailure,"grammars",observations,"effective_rules",effective,"effective_failure",effectiveFailure,"provider",fields("logical_resource_uri",provider,"namespace",pkg.getNsURI()))));
 }
}
