// Independent verification adapter. Never installed in production.
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
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.resource.XtextResourceSet;
import org.eclipse.xtext.validation.IResourceValidator;

public final class OrdinaryParameterOracle {
  static Map<String,Object> span(EObject object) {
    var n=NodeModelUtils.getNode(object);
    return fields("start",n==null?null:n.getOffset(),"end",n==null?null:n.getEndOffset());
  }
  static Object parameter(Parameter p) {
    if(p==null)return null;
    var owner=p.eContainer() instanceof ParserRule?(ParserRule)p.eContainer():null;
    var grammar=owner==null?null:GrammarUtil.getGrammar(owner);
    return fields("name",p.getName(),"grammar",grammar==null?null:grammar.getName(),
      "rule",owner==null?null:owner.getName(),"position",owner==null?null:owner.getParameters().indexOf(p),
      "proxy",p.eIsProxy(),"uri",EcoreUtil.getURI(p).toString(),"span",span(p));
  }
  static Object condition(Condition c) {
    if(c==null)return null;
    Map<String,Object> row=fields("kind",c.eClass().getName(),"span",span(c));
    if(c instanceof LiteralCondition)row.put("value",((LiteralCondition)c).isTrue());
    else if(c instanceof ParameterReference)row.put("parameter",parameter(((ParameterReference)c).getParameter()));
    else if(c instanceof Negation)row.put("value",condition(((Negation)c).getValue()));
    else if(c instanceof CompositeCondition) {
      row.put("left",condition(((CompositeCondition)c).getLeft()));
      row.put("right",condition(((CompositeCondition)c).getRight()));
    } else throw new IllegalArgumentException("Unobserved condition kind "+c.eClass().getName());
    return row;
  }
  static Map<String,Object> observe(String id,Resource resource,IResourceValidator validator) {
    Map<String,Object> row=FragmentOracle.snapshot(id,resource,validator);
    Grammar grammar=(Grammar)resource.getContents().get(0);
    List<Object> bindings=new ArrayList<>();
    for(var rule:grammar.getRules())if(rule instanceof ParserRule) {
      ParserRule parser=(ParserRule)rule;
      List<Object> parameters=new ArrayList<>(),calls=new ArrayList<>(),guards=new ArrayList<>();
      for(var p:parser.getParameters())parameters.add(parameter(p));
      var nodes=parser.eAllContents();
      while(nodes.hasNext()) {
        EObject node=nodes.next();
        if(node instanceof RuleCall) {
          RuleCall call=(RuleCall)node;List<Object> arguments=new ArrayList<>();
          for(var argument:call.getArguments())arguments.add(fields("by_name",argument.isCalledByName(),
            "parameter",parameter(argument.getParameter()),"value",condition(argument.getValue()),"span",span(argument)));
          calls.add(fields("target",FragmentOracle.rule(call.getRule()),"arguments",arguments,"span",span(call)));
        }
        if(node instanceof Group && ((Group)node).getGuardCondition()!=null)
          guards.add(fields("condition",condition(((Group)node).getGuardCondition()),"span",span(node)));
      }
      bindings.add(fields("rule",parser.getName(),"span",span(parser),"parameters",parameters,"calls",calls,"guards",guards));
    }
    row.put("parameter_bindings",bindings);return row;
  }
  public static void main(String[] args)throws Exception {
    var injector=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();
    var validator=injector.getInstance(IResourceValidator.class);
    Path root=Path.of(args[0]).toAbsolutePath();
    for(int index=1;index<args.length;index++) {
      String name=args[index];Path dir=root.resolve(name);
      var resources=injector.getInstance(XtextResourceSet.class);
      List<EPackage> packages=new ArrayList<>();List<Resource> modelResources=new ArrayList<>();
      for(String file:Files.readAllLines(dir.resolve("models.txt"))) {
        var resource=resources.createResource(URI.createURI("memory:/ordinary-parameters/"+name+"/"+file));
        resource.load(new ByteArrayInputStream(Files.readAllBytes(dir.resolve(file))),null);
        var p=(EPackage)resource.getContents().get(0);packages.add(p);modelResources.add(resource);
        resources.getPackageRegistry().put(p.getNsURI(),p);
      }
      for(var resource:modelResources)EcoreUtil.resolveAll(resource);
      List<Object> before=new ArrayList<>();for(var p:packages)before.add(metamodel(p));
      Map<String,Resource> loaded=new LinkedHashMap<>();List<Object> initial=new ArrayList<>();
      for(String id:Files.readAllLines(dir.resolve("order.txt"))) {
        var resource=resources.createResource(URI.createURI("memory:/ordinary-parameters/"+name+"/"+id+".xtext"));
        resource.load(new ByteArrayInputStream(Files.readAllBytes(dir.resolve(id+".xtext"))),null);
        EcoreUtil.resolveAll(resource);initial.add(observe(id,resource,validator));loaded.put(id,resource);
      }
      List<Object> complete=new ArrayList<>();boolean all=true;
      for(var item:loaded.entrySet()) {
        EcoreUtil.resolveAll(item.getValue());var row=observe(item.getKey(),item.getValue(),validator);
        complete.add(row);all&=(Boolean)row.get("accepted");
      }
      List<Object> after=new ArrayList<>();for(var p:packages)after.add(metamodel(p));
      System.out.println(json(fields("id",name,"all_sources_accepted",all,"initial",initial,"sources",complete,
        "imported_before",before,"imported_after",after,
        "imported_metadata_unchanged",json(before).equals(json(after)),
        "imported_structure_unchanged",json(FragmentOracle.structural(before)).equals(json(FragmentOracle.structural(after))))));
    }
  }
}
