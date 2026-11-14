// Independently authored verification adapter. Java is never used by production.
package intersect.oracle;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.xtext.*;
import org.eclipse.xtext.nodemodel.util.NodeModelUtils;
import org.eclipse.xtext.resource.XtextResourceSet;
import org.eclipse.xtext.util.CancelIndicator;
import org.eclipse.xtext.validation.CheckMode;
import org.eclipse.xtext.validation.IResourceValidator;
import org.eclipse.xtext.xtext.ecoreInference.SourceAdapter;

public final class MetamodelOracle {
  static Map<String,Object> fields(Object... values) {
    Map<String,Object> result=new LinkedHashMap<>();
    for(int i=0;i<values.length;i+=2) result.put((String)values[i],values[i+1]);
    return result;
  }
  static String quote(String text) {
    if(text==null) return "null";
    StringBuilder result=new StringBuilder("\"");
    for(char c:text.toCharArray()) {
      switch(c) {
        case '\\': result.append("\\\\");break;
        case '\"': result.append("\\\"");break;
        case '\n': result.append("\\n");break;
        case '\r': result.append("\\r");break;
        case '\t': result.append("\\t");break;
        default: if(c<32)result.append(String.format("\\u%04x",(int)c));else result.append(c);
      }
    }
    return result.append('\"').toString();
  }
  static String json(Object value) {
    if(value==null) return "null";
    if(value instanceof String)return quote((String)value);
    if(value instanceof Number || value instanceof Boolean)return value.toString();
    if(value instanceof Map<?,?>) {
      StringJoiner result=new StringJoiner(",","{","}");
      ((Map<?,?>)value).forEach((key,item)->result.add(quote((String)key)+":"+json(item)));
      return result.toString();
    }
    if(value instanceof Iterable<?>) {
      StringJoiner result=new StringJoiner(",","[","]");
      for(Object item:(Iterable<?>)value)result.add(json(item));
      return result.toString();
    }
    throw new IllegalArgumentException("Unsupported observation value "+value.getClass());
  }
  static String identity(EClassifier classifier) {
    if(classifier==null)return null;
    if(classifier.eIsProxy())return "proxy:"+((org.eclipse.emf.ecore.InternalEObject)classifier).eProxyURI();
    EPackage owner=classifier.getEPackage();
    return (owner==null?"uncontained":owner.getNsURI())+"#//"+classifier.getName();
  }
  static List<Object> origins(EObject value) {
    List<Object> result=new ArrayList<>();
    SourceAdapter adapter=SourceAdapter.find(value);
    if(adapter!=null) for(EObject source:adapter.getSources()) {
      var node=NodeModelUtils.getNode(source);
      if(node!=null) {
        Grammar grammar=GrammarUtil.getGrammar(source);
        result.add(fields("grammar",grammar==null?null:grammar.getName(),
          "kind",source.eClass().getName(),"start",node.getOffset(),"end",node.getEndOffset()));
      }
    }
    result.sort(Comparator.comparing(MetamodelOracle::json));
    return result;
  }
  static List<Object> annotations(EModelElement value) {
    List<Object> result=new ArrayList<>();
    for(EAnnotation annotation:value.getEAnnotations()) {
      Map<String,Object> details=new TreeMap<>();
      for(var detail:annotation.getDetails())details.put(detail.getKey(),detail.getValue());
      result.add(fields("source",annotation.getSource(),"details",details));
    }
    return result;
  }
  static Map<String,Object> feature(EStructuralFeature f) {
    Map<String,Object> result=fields("name",f.getName(),"kind",f.eClass().getName(),
      "type",identity(f.getEType()),"lower",f.getLowerBound(),"upper",f.getUpperBound(),
      "ordered",f.isOrdered(),"unique",f.isUnique(),"changeable",f.isChangeable(),
      "volatile",f.isVolatile(),"transient",f.isTransient(),"derived",f.isDerived(),
      "unsettable",f.isUnsettable(),"default_literal",f.getDefaultValueLiteral(),
      "annotations",annotations(f),"origins",origins(f));
    if(f instanceof EReference) {
      EReference ref=(EReference)f;
      EReference opposite=ref.getEOpposite();
      List<String> keys=new ArrayList<>();for(EAttribute key:ref.getEKeys())keys.add(key.getName());
      result.put("containment",ref.isContainment());result.put("container",ref.isContainer());
      result.put("resolve_proxies",ref.isResolveProxies());
      result.put("opposite",opposite==null?null:identity(opposite.getEContainingClass())+"/"+opposite.getName());
      result.put("keys",keys);
    } else result.put("id",((EAttribute)f).isID());
    return result;
  }
  static Map<String,Object> classifier(EClassifier c) {
    Map<String,Object> result=fields("name",c.getName(),"identity",identity(c),"kind",c.eClass().getName(),
      "instance_class",c.getInstanceClassName(),"instance_type",c.getInstanceTypeName(),
      "annotations",annotations(c),"origins",origins(c));
    if(c instanceof EClass) {
      EClass cls=(EClass)c;
      List<String> parents=new ArrayList<>();for(EClass parent:cls.getESuperTypes())parents.add(identity(parent));
      List<Object> features=new ArrayList<>();for(var feature:cls.getEStructuralFeatures())features.add(feature(feature));
      result.put("abstract",cls.isAbstract());result.put("interface",cls.isInterface());
      result.put("super_types",parents);result.put("features",features);
    } else {
      result.put("serializable",((EDataType)c).isSerializable());
      if(c instanceof EEnum) {
        List<Object> literals=new ArrayList<>();
        for(var literal:((EEnum)c).getELiterals())literals.add(fields("name",literal.getName(),
          "literal",literal.getLiteral(),"value",literal.getValue(),"origins",origins(literal)));
        result.put("literals",literals);
      }
    }
    return result;
  }
  static Map<String,Object> metamodel(EPackage p) {
    List<Object> classifiers=new ArrayList<>();for(EClassifier c:p.getEClassifiers())classifiers.add(classifier(c));
    return fields("name",p.getName(),"uri",p.getNsURI(),"prefix",p.getNsPrefix(),
      "annotations",annotations(p),"classifiers",classifiers);
  }
  public static void main(String[] args)throws Exception {
    var injector=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration();
    IResourceValidator validator=injector.getInstance(IResourceValidator.class);
    Path directory=Path.of(args[0]).toAbsolutePath();
    for(int i=1;i<args.length;i++) {
      Path path=directory.resolve(args[i]+".xtext");
      XtextResourceSet resources=injector.getInstance(XtextResourceSet.class);
      var importedResource=resources.getResource(URI.createFileURI(directory.resolve("imported.ecore").toString()),true);
      EPackage imported=(EPackage)importedResource.getContents().get(0);
      resources.getPackageRegistry().put(imported.getNsURI(),imported);
      String before=json(metamodel(imported));
      var resource=resources.createResource(URI.createFileURI(path.toString()));
      resource.load(new ByteArrayInputStream(Files.readAllBytes(path)),null);
      EcoreUtil.resolveAll(resource);
      List<Object> diagnostics=new ArrayList<>();
      int errors=0;
      for(var issue:validator.validate(resource,CheckMode.ALL,CancelIndicator.NullImpl)) {
        if(issue.getSeverity()==org.eclipse.xtext.diagnostics.Severity.ERROR)errors++;
        diagnostics.add(fields("severity",issue.getSeverity().name(),"code",issue.getCode(),
          "start",issue.getOffset(),"length",issue.getLength(),"message",issue.getMessage()));
      }
      diagnostics.sort(Comparator.comparing(MetamodelOracle::json));
      Grammar grammar=(Grammar)resource.getContents().get(0);
      List<Object> generated=new ArrayList<>();
      for(var declaration:grammar.getMetamodelDeclarations())if(declaration instanceof GeneratedMetamodel) {
        EPackage p=declaration.getEPackage();
        generated.add(fields("alias",declaration.getAlias(),"package",p==null||p.eIsProxy()?null:metamodel(p)));
      }
      Map<String,Object> after=metamodel(imported);
      System.out.println(json(fields("id",args[i],"accepted",errors==0,"error_count",errors,
        "generated_packages",generated,"imported_unchanged",before.equals(json(after)),
        "imported_package",after,"diagnostics",diagnostics)));
    }
  }
}
