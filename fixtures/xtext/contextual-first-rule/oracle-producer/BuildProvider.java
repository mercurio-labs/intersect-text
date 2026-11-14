import java.nio.file.*;
import java.util.*;
import org.eclipse.emf.common.util.URI;
import org.eclipse.emf.common.util.BasicMonitor;
import org.eclipse.emf.ecore.*;
import org.eclipse.emf.ecore.resource.impl.ResourceSetImpl;
import org.eclipse.emf.ecore.xmi.impl.EcoreResourceFactoryImpl;
import org.eclipse.emf.ecore.xmi.impl.XMIResourceFactoryImpl;
import org.eclipse.emf.ecore.plugin.EcorePlugin;
import org.eclipse.emf.codegen.ecore.genmodel.*;
import org.eclipse.emf.codegen.ecore.generator.Generator;
import org.eclipse.emf.codegen.ecore.genmodel.generator.*;
public class BuildProvider {
 public static void main(String[] args)throws Exception{
  Path root=Path.of(args[1]).toAbsolutePath();Files.createDirectories(root);
  var set=new ResourceSetImpl();set.getResourceFactoryRegistry().getExtensionToFactoryMap().put("ecore",new EcoreResourceFactoryImpl());set.getResourceFactoryRegistry().getExtensionToFactoryMap().put("genmodel",new XMIResourceFactoryImpl());
  var source=set.getResource(URI.createFileURI(Path.of(args[0]).toAbsolutePath().toString()),true);var pkg=(EPackage)source.getContents().get(0);
  var gm=GenModelFactory.eINSTANCE.createGenModel();gm.setModelName("LexicalOracle");gm.setModelDirectory("/lexical-provider/src-gen");gm.setComplianceLevel(GenJDKLevel.JDK170_LITERAL);gm.initialize(List.of(pkg));gm.setCanGenerate(true);
  for(var gp:gm.getGenPackages())gp.setBasePackage("oracle.provider");
  var resource=set.createResource(URI.createFileURI(root.resolve("provider.genmodel").toString()));resource.getContents().add(gm);resource.save(null);
  EcorePlugin.getPlatformResourceMap().put("lexical-provider",URI.createFileURI(root.toString()+"/"));
  org.eclipse.emf.codegen.ecore.generator.GeneratorAdapterFactory.Descriptor.Registry.INSTANCE.addDescriptor(GenModelPackage.eNS_URI,GenModelGeneratorAdapterFactory.DESCRIPTOR);
  var generator=new Generator();generator.setInput(gm);var result=generator.generate(gm,GenBaseGeneratorAdapter.MODEL_PROJECT_TYPE,new BasicMonitor.Printing(System.out));System.out.println("DIAGNOSTIC="+result);
  System.out.println("PACKAGE_CLASS="+gm.getGenPackages().get(0).getQualifiedPackageInterfaceName());if(result.getSeverity()>=4)System.exit(1);
 }
}
