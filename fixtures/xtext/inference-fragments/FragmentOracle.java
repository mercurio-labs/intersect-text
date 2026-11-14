// Independently authored oracle adapter; production never loads Java.
package intersect.oracle;
import static intersect.oracle.MetamodelOracle.*;
import java.io.ByteArrayInputStream;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.resource.Resource;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.xtext.*;
import org.eclipse.xtext.resource.XtextResourceSet;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.util.CancelIndicator;
import org.eclipse.xtext.validation.*;

public final class FragmentOracle {
  // Source loading follows CompositionOracle; metadata serialization reuses
  // the unchanged MetamodelOracle. Parser flags retain exact rule categories.
  // Default cross-reference rule calls can be synthesized by Xtext and have no
  // authored node. Retain them explicitly with null spans instead of dropping them.
  static Map<String,Object> snapshot(String id,Resource resource,IResourceValidator validator) {
    Grammar grammar=(Grammar)resource.getContents().get(0);
    List<Object> diagnostics=new ArrayList<>();int errors=0;
    for(var issue:validator.validate(resource,CheckMode.ALL,CancelIndicator.NullImpl)) {
      if(issue.getSeverity()==org.eclipse.xtext.diagnostics.Severity.ERROR)errors++;
      diagnostics.add(fields("severity",issue.getSeverity().name(),"code",issue.getCode(),
        "start",issue.getOffset(),"length",issue.getLength(),"message",issue.getMessage()));
    }
    diagnostics.sort(Comparator.comparing(MetamodelOracle::json));
    List<Object> parents=new ArrayList<>();
    for(var parent:grammar.getUsedGrammars())parents.add(fields("name",parent.getName(),"proxy",parent.eIsProxy()));
    List<Object> rules=new ArrayList<>();
    for(var declared:grammar.getRules()) {
      Map<String,Object> info=rule(declared);flags(info,declared);List<Object> calls=new ArrayList<>();
      var nodeSpan=NodeModelUtils.getNode(declared);
      info.put("start",nodeSpan.getOffset());info.put("end",nodeSpan.getEndOffset());
      var nodes=declared.eAllContents();
      while(nodes.hasNext()) {
        var node=nodes.next();
        if(node instanceof RuleCall) {
          var call=(RuleCall)node;var target=call.getRule();var span=NodeModelUtils.getNode(call);
          var targetInfo=rule(target);flags(targetInfo,target);
          calls.add(fields("target",targetInfo,"explicit",call.isExplicitlyCalled(),
            "proxy",target!=null&&target.eIsProxy(),"synthetic",span==null,
            "start",span==null?null:span.getOffset(),"end",span==null?null:span.getEndOffset()));
        }
      }
      info.put("calls",calls);rules.add(info);
    }
    return fields("id",id,"grammar",grammar.getName(),"accepted",errors==0,"error_count",errors,
      "parents",parents,"metamodels",packages(grammar),"rules",rules,"diagnostics",diagnostics);
  }
  static void flags(Map<String,Object> row,AbstractRule rule) {
    row.put("fragment",rule instanceof ParserRule && ((ParserRule)rule).isFragment());
    row.put("wildcard",rule instanceof ParserRule && ((ParserRule)rule).isWildcard());
  }

  static List<Object> packages(Grammar grammar) {
    List<Object> result=new ArrayList<>();
    for(var declaration:grammar.getMetamodelDeclarations()) {
      var p=declaration.getEPackage();
      result.add(fields("generated",declaration instanceof GeneratedMetamodel,
        "alias",declaration.getAlias(),"namespace",p==null?null:p.getNsURI(),
        "package",p==null||p.eIsProxy()?null:declaration instanceof GeneratedMetamodel?metamodel(p):fields("name",p.getName(),"uri",p.getNsURI(),"prefix",p.getNsPrefix())));
    }
    return result;
  }
  static Map<String,Object> rule(AbstractRule rule) {
    if(rule==null)return fields("name",null,"grammar",null,"kind",null,"type",null);
    var ref=rule.getType();var owner=GrammarUtil.getGrammar(rule);
    return fields("name",rule.getName(),"grammar",owner==null?null:owner.getName(),
      "kind",rule.eClass().getName(),"type",ref==null?null:identity(ref.getClassifier()));
  }
  static Object structural(Object value) {
    if(value instanceof Map<?,?>) {
      Map<String,Object> result=new LinkedHashMap<>();
      ((Map<?,?>)value).forEach((key,item)->{if(!key.equals("origins"))result.put((String)key,structural(item));});
      return result;
    }
    if(value instanceof Iterable<?>) {
      List<Object> result=new ArrayList<>();for(Object item:(Iterable<?>)value)result.add(structural(item));return result;
    }
    return value;
  }
  public static void main(String[] args)throws Exception {
    var injector=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();
    var validator=injector.getInstance(IResourceValidator.class);
    Path root=Path.of(args[0]).toAbsolutePath();
    for(int index=1;index<args.length;index++) {
      String name=args[index];Path dir=root.resolve(name);
      var resources=injector.getInstance(XtextResourceSet.class);
      var importedResource=resources.getResource(URI.createFileURI(root.resolve("imported.ecore").toString()),true);
      EPackage imported=(EPackage)importedResource.getContents().get(0);
      resources.getPackageRegistry().put(imported.getNsURI(),imported);
      String importedBefore=json(metamodel(imported));
      Map<String,Resource> loaded=new LinkedHashMap<>();
      List<Object> initial=new ArrayList<>();
      for(String id:Files.readAllLines(dir.resolve("order.txt"))) {
        var resource=resources.createResource(URI.createURI("memory:/"+name+"/"+id+".xtext"));
        resource.load(new ByteArrayInputStream(Files.readAllBytes(dir.resolve(id+".xtext"))),null);
        EcoreUtil.resolveAll(resource);
        initial.add(snapshot(id,resource,validator));loaded.put(id,resource);
      }
      List<Object> complete=new ArrayList<>();boolean all=true;boolean entry=false;
      String entryId=Files.readString(dir.resolve("entry.txt")).trim();
      for(var item:loaded.entrySet()) {
        EcoreUtil.resolveAll(item.getValue());var observed=snapshot(item.getKey(),item.getValue(),validator);
        complete.add(observed);all&=(Boolean)observed.get("accepted");
        if(item.getKey().equals(entryId))entry=(Boolean)observed.get("accepted");
      }
      List<Object> immutable=new ArrayList<>();
      for(int i=0;i<initial.size()-1;i++) {
        @SuppressWarnings("unchecked") var before=(Map<String,Object>)initial.get(i);
        @SuppressWarnings("unchecked") var after=(Map<String,Object>)complete.get(i);
        boolean metadata=json(before.get("metamodels")).equals(json(after.get("metamodels")));
        boolean structure=json(structural(before.get("metamodels"))).equals(json(structural(after.get("metamodels"))));
        immutable.add(fields("id",before.get("id"),"metadata_unchanged",metadata,"structure_unchanged",structure));
      }
      System.out.println(json(fields("id",name,"entry",entryId,"accepted",entry,
        "all_sources_accepted",all,"initial",initial,"sources",complete,"parent_snapshots",immutable,
        "imported_unchanged",importedBefore.equals(json(metamodel(imported))),"imported_package",metamodel(imported))));
    }
  }
}
