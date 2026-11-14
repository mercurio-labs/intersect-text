/**
 */
package oracle.provider.colors.impl;

import oracle.provider.colors.ColorsFactory;
import oracle.provider.colors.ColorsPackage;
import oracle.provider.colors.Model;
import oracle.provider.colors.OtherShade;
import oracle.provider.colors.Shade;

import org.eclipse.emf.ecore.EAttribute;
import org.eclipse.emf.ecore.EClass;
import org.eclipse.emf.ecore.EEnum;
import org.eclipse.emf.ecore.EPackage;

import org.eclipse.emf.ecore.impl.EPackageImpl;

/**
 * <!-- begin-user-doc -->
 * An implementation of the model <b>Package</b>.
 * <!-- end-user-doc -->
 * @generated
 */
public class ColorsPackageImpl extends EPackageImpl implements ColorsPackage {
	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	private EClass modelEClass = null;

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	private EEnum shadeEEnum = null;

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	private EEnum otherShadeEEnum = null;

	/**
	 * Creates an instance of the model <b>Package</b>, registered with
	 * {@link org.eclipse.emf.ecore.EPackage.Registry EPackage.Registry} by the package
	 * package URI value.
	 * <p>Note: the correct way to create the package is via the static
	 * factory method {@link #init init()}, which also performs
	 * initialization of the package, or returns the registered package,
	 * if one already exists.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see org.eclipse.emf.ecore.EPackage.Registry
	 * @see oracle.provider.colors.ColorsPackage#eNS_URI
	 * @see #init()
	 * @generated
	 */
	private ColorsPackageImpl() {
		super(eNS_URI, ColorsFactory.eINSTANCE);
	}
	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	private static boolean isInited = false;

	/**
	 * Creates, registers, and initializes the <b>Package</b> for this model, and for any others upon which it depends.
	 *
	 * <p>This method is used to initialize {@link ColorsPackage#eINSTANCE} when that field is accessed.
	 * Clients should not invoke it directly. Instead, they should simply access that field to obtain the package.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see #eNS_URI
	 * @see #createPackageContents()
	 * @see #initializePackageContents()
	 * @generated
	 */
	public static ColorsPackage init() {
		if (isInited) return (ColorsPackage)EPackage.Registry.INSTANCE.getEPackage(ColorsPackage.eNS_URI);

		// Obtain or create and register package
		Object registeredColorsPackage = EPackage.Registry.INSTANCE.get(eNS_URI);
		ColorsPackageImpl theColorsPackage = registeredColorsPackage instanceof ColorsPackageImpl ? (ColorsPackageImpl)registeredColorsPackage : new ColorsPackageImpl();

		isInited = true;

		// Create package meta-data objects
		theColorsPackage.createPackageContents();

		// Initialize created meta-data
		theColorsPackage.initializePackageContents();

		// Mark meta-data to indicate it can't be changed
		theColorsPackage.freeze();

		// Update the registry and return the package
		EPackage.Registry.INSTANCE.put(ColorsPackage.eNS_URI, theColorsPackage);
		return theColorsPackage;
	}

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	@Override
	public EClass getModel() {
		return modelEClass;
	}

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	@Override
	public EAttribute getModel_Shade() {
		return (EAttribute)modelEClass.getEStructuralFeatures().get(0);
	}

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	@Override
	public EEnum getShade() {
		return shadeEEnum;
	}

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	@Override
	public EEnum getOtherShade() {
		return otherShadeEEnum;
	}

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	@Override
	public ColorsFactory getColorsFactory() {
		return (ColorsFactory)getEFactoryInstance();
	}

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	private boolean isCreated = false;

	/**
	 * Creates the meta-model objects for the package.  This method is
	 * guarded to have no affect on any invocation but its first.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	public void createPackageContents() {
		if (isCreated) return;
		isCreated = true;

		// Create classes and their features
		modelEClass = createEClass(MODEL);
		createEAttribute(modelEClass, MODEL__SHADE);

		// Create enums
		shadeEEnum = createEEnum(SHADE);
		otherShadeEEnum = createEEnum(OTHER_SHADE);
	}

	/**
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	private boolean isInitialized = false;

	/**
	 * Complete the initialization of the package and its meta-model.  This
	 * method is guarded to have no affect on any invocation but its first.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	public void initializePackageContents() {
		if (isInitialized) return;
		isInitialized = true;

		// Initialize package
		setName(eNAME);
		setNsPrefix(eNS_PREFIX);
		setNsURI(eNS_URI);

		// Create type parameters

		// Set bounds for type parameters

		// Add supertypes to classes

		// Initialize classes and features; add operations and parameters
		initEClass(modelEClass, Model.class, "Model", !IS_ABSTRACT, !IS_INTERFACE, IS_GENERATED_INSTANCE_CLASS);
		initEAttribute(getModel_Shade(), this.getShade(), "shade", null, 0, 1, Model.class, !IS_TRANSIENT, !IS_VOLATILE, IS_CHANGEABLE, !IS_UNSETTABLE, !IS_ID, IS_UNIQUE, !IS_DERIVED, IS_ORDERED);

		// Initialize enums and add enum literals
		initEEnum(shadeEEnum, Shade.class, "Shade");
		addEEnumLiteral(shadeEEnum, Shade.RED);
		addEEnumLiteral(shadeEEnum, Shade.BLUE);

		initEEnum(otherShadeEEnum, OtherShade.class, "OtherShade");
		addEEnumLiteral(otherShadeEEnum, OtherShade.RED);
		addEEnumLiteral(otherShadeEEnum, OtherShade.BLUE);

		// Create resource
		createResource(eNS_URI);
	}

} //ColorsPackageImpl
