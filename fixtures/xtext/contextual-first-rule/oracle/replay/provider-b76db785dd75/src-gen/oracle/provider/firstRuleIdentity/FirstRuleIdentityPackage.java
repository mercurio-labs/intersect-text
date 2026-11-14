/**
 */
package oracle.provider.firstRuleIdentity;

import org.eclipse.emf.ecore.EAttribute;
import org.eclipse.emf.ecore.EClass;
import org.eclipse.emf.ecore.EEnum;
import org.eclipse.emf.ecore.EPackage;
import org.eclipse.emf.ecore.EReference;

/**
 * <!-- begin-user-doc -->
 * The <b>Package</b> for the model.
 * It contains accessors for the meta objects to represent
 * <ul>
 *   <li>each class,</li>
 *   <li>each feature of each class,</li>
 *   <li>each enum,</li>
 *   <li>and each data type</li>
 * </ul>
 * <!-- end-user-doc -->
 * @see oracle.provider.firstRuleIdentity.FirstRuleIdentityFactory
 * @model kind="package"
 * @generated
 */
public interface FirstRuleIdentityPackage extends EPackage {
	/**
	 * The package name.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	String eNAME = "firstRuleIdentity";

	/**
	 * The package namespace URI.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	String eNS_URI = "urn:intersect:xtext:first-rule:identity:1";

	/**
	 * The package namespace name.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	String eNS_PREFIX = "k";

	/**
	 * The singleton instance of the package.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	FirstRuleIdentityPackage eINSTANCE = oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl.init();

	/**
	 * The meta object id for the '{@link oracle.provider.firstRuleIdentity.impl.ModelImpl <em>Model</em>}' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.firstRuleIdentity.impl.ModelImpl
	 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getModel()
	 * @generated
	 */
	int MODEL = 0;

	/**
	 * The feature id for the '<em><b>Name</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int MODEL__NAME = 0;

	/**
	 * The feature id for the '<em><b>Members</b></em>' containment reference list.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int MODEL__MEMBERS = 1;

	/**
	 * The number of structural features of the '<em>Model</em>' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int MODEL_FEATURE_COUNT = 2;

	/**
	 * The meta object id for the '{@link oracle.provider.firstRuleIdentity.impl.ElementImpl <em>Element</em>}' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.firstRuleIdentity.impl.ElementImpl
	 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getElement()
	 * @generated
	 */
	int ELEMENT = 1;

	/**
	 * The feature id for the '<em><b>Name</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int ELEMENT__NAME = 0;

	/**
	 * The number of structural features of the '<em>Element</em>' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int ELEMENT_FEATURE_COUNT = 1;

	/**
	 * The meta object id for the '{@link oracle.provider.firstRuleIdentity.impl.TypeImpl <em>Type</em>}' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.firstRuleIdentity.impl.TypeImpl
	 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getType()
	 * @generated
	 */
	int TYPE = 2;

	/**
	 * The feature id for the '<em><b>Name</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int TYPE__NAME = ELEMENT__NAME;

	/**
	 * The feature id for the '<em><b>Is Abstract</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int TYPE__IS_ABSTRACT = ELEMENT_FEATURE_COUNT + 0;

	/**
	 * The number of structural features of the '<em>Type</em>' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int TYPE_FEATURE_COUNT = ELEMENT_FEATURE_COUNT + 1;

	/**
	 * The meta object id for the '{@link oracle.provider.firstRuleIdentity.impl.FeatureImpl <em>Feature</em>}' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.firstRuleIdentity.impl.FeatureImpl
	 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getFeature()
	 * @generated
	 */
	int FEATURE = 3;

	/**
	 * The feature id for the '<em><b>Name</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int FEATURE__NAME = ELEMENT__NAME;

	/**
	 * The feature id for the '<em><b>Target</b></em>' reference.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int FEATURE__TARGET = ELEMENT_FEATURE_COUNT + 0;

	/**
	 * The number of structural features of the '<em>Feature</em>' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int FEATURE_FEATURE_COUNT = ELEMENT_FEATURE_COUNT + 1;

	/**
	 * The meta object id for the '{@link oracle.provider.firstRuleIdentity.impl.EStringImpl <em>EString</em>}' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.firstRuleIdentity.impl.EStringImpl
	 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getEString()
	 * @generated
	 */
	int ESTRING = 4;

	/**
	 * The feature id for the '<em><b>Name</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int ESTRING__NAME = MODEL__NAME;

	/**
	 * The feature id for the '<em><b>Members</b></em>' containment reference list.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int ESTRING__MEMBERS = MODEL__MEMBERS;

	/**
	 * The number of structural features of the '<em>EString</em>' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int ESTRING_FEATURE_COUNT = MODEL_FEATURE_COUNT + 0;

	/**
	 * The meta object id for the '{@link oracle.provider.firstRuleIdentity.Choice <em>Choice</em>}' enum.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.firstRuleIdentity.Choice
	 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getChoice()
	 * @generated
	 */
	int CHOICE = 5;


	/**
	 * Returns the meta object for class '{@link oracle.provider.firstRuleIdentity.Model <em>Model</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for class '<em>Model</em>'.
	 * @see oracle.provider.firstRuleIdentity.Model
	 * @generated
	 */
	EClass getModel();

	/**
	 * Returns the meta object for the attribute '{@link oracle.provider.firstRuleIdentity.Model#getName <em>Name</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for the attribute '<em>Name</em>'.
	 * @see oracle.provider.firstRuleIdentity.Model#getName()
	 * @see #getModel()
	 * @generated
	 */
	EAttribute getModel_Name();

	/**
	 * Returns the meta object for the containment reference list '{@link oracle.provider.firstRuleIdentity.Model#getMembers <em>Members</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for the containment reference list '<em>Members</em>'.
	 * @see oracle.provider.firstRuleIdentity.Model#getMembers()
	 * @see #getModel()
	 * @generated
	 */
	EReference getModel_Members();

	/**
	 * Returns the meta object for class '{@link oracle.provider.firstRuleIdentity.Element <em>Element</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for class '<em>Element</em>'.
	 * @see oracle.provider.firstRuleIdentity.Element
	 * @generated
	 */
	EClass getElement();

	/**
	 * Returns the meta object for the attribute '{@link oracle.provider.firstRuleIdentity.Element#getName <em>Name</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for the attribute '<em>Name</em>'.
	 * @see oracle.provider.firstRuleIdentity.Element#getName()
	 * @see #getElement()
	 * @generated
	 */
	EAttribute getElement_Name();

	/**
	 * Returns the meta object for class '{@link oracle.provider.firstRuleIdentity.Type <em>Type</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for class '<em>Type</em>'.
	 * @see oracle.provider.firstRuleIdentity.Type
	 * @generated
	 */
	EClass getType();

	/**
	 * Returns the meta object for the attribute '{@link oracle.provider.firstRuleIdentity.Type#isIsAbstract <em>Is Abstract</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for the attribute '<em>Is Abstract</em>'.
	 * @see oracle.provider.firstRuleIdentity.Type#isIsAbstract()
	 * @see #getType()
	 * @generated
	 */
	EAttribute getType_IsAbstract();

	/**
	 * Returns the meta object for class '{@link oracle.provider.firstRuleIdentity.Feature <em>Feature</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for class '<em>Feature</em>'.
	 * @see oracle.provider.firstRuleIdentity.Feature
	 * @generated
	 */
	EClass getFeature();

	/**
	 * Returns the meta object for the reference '{@link oracle.provider.firstRuleIdentity.Feature#getTarget <em>Target</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for the reference '<em>Target</em>'.
	 * @see oracle.provider.firstRuleIdentity.Feature#getTarget()
	 * @see #getFeature()
	 * @generated
	 */
	EReference getFeature_Target();

	/**
	 * Returns the meta object for class '{@link oracle.provider.firstRuleIdentity.EString <em>EString</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for class '<em>EString</em>'.
	 * @see oracle.provider.firstRuleIdentity.EString
	 * @generated
	 */
	EClass getEString();

	/**
	 * Returns the meta object for enum '{@link oracle.provider.firstRuleIdentity.Choice <em>Choice</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for enum '<em>Choice</em>'.
	 * @see oracle.provider.firstRuleIdentity.Choice
	 * @generated
	 */
	EEnum getChoice();

	/**
	 * Returns the factory that creates the instances of the model.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the factory that creates the instances of the model.
	 * @generated
	 */
	FirstRuleIdentityFactory getFirstRuleIdentityFactory();

	/**
	 * <!-- begin-user-doc -->
	 * Defines literals for the meta objects that represent
	 * <ul>
	 *   <li>each class,</li>
	 *   <li>each feature of each class,</li>
	 *   <li>each enum,</li>
	 *   <li>and each data type</li>
	 * </ul>
	 * <!-- end-user-doc -->
	 * @generated
	 */
	interface Literals {
		/**
		 * The meta object literal for the '{@link oracle.provider.firstRuleIdentity.impl.ModelImpl <em>Model</em>}' class.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.firstRuleIdentity.impl.ModelImpl
		 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getModel()
		 * @generated
		 */
		EClass MODEL = eINSTANCE.getModel();

		/**
		 * The meta object literal for the '<em><b>Name</b></em>' attribute feature.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @generated
		 */
		EAttribute MODEL__NAME = eINSTANCE.getModel_Name();

		/**
		 * The meta object literal for the '<em><b>Members</b></em>' containment reference list feature.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @generated
		 */
		EReference MODEL__MEMBERS = eINSTANCE.getModel_Members();

		/**
		 * The meta object literal for the '{@link oracle.provider.firstRuleIdentity.impl.ElementImpl <em>Element</em>}' class.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.firstRuleIdentity.impl.ElementImpl
		 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getElement()
		 * @generated
		 */
		EClass ELEMENT = eINSTANCE.getElement();

		/**
		 * The meta object literal for the '<em><b>Name</b></em>' attribute feature.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @generated
		 */
		EAttribute ELEMENT__NAME = eINSTANCE.getElement_Name();

		/**
		 * The meta object literal for the '{@link oracle.provider.firstRuleIdentity.impl.TypeImpl <em>Type</em>}' class.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.firstRuleIdentity.impl.TypeImpl
		 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getType()
		 * @generated
		 */
		EClass TYPE = eINSTANCE.getType();

		/**
		 * The meta object literal for the '<em><b>Is Abstract</b></em>' attribute feature.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @generated
		 */
		EAttribute TYPE__IS_ABSTRACT = eINSTANCE.getType_IsAbstract();

		/**
		 * The meta object literal for the '{@link oracle.provider.firstRuleIdentity.impl.FeatureImpl <em>Feature</em>}' class.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.firstRuleIdentity.impl.FeatureImpl
		 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getFeature()
		 * @generated
		 */
		EClass FEATURE = eINSTANCE.getFeature();

		/**
		 * The meta object literal for the '<em><b>Target</b></em>' reference feature.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @generated
		 */
		EReference FEATURE__TARGET = eINSTANCE.getFeature_Target();

		/**
		 * The meta object literal for the '{@link oracle.provider.firstRuleIdentity.impl.EStringImpl <em>EString</em>}' class.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.firstRuleIdentity.impl.EStringImpl
		 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getEString()
		 * @generated
		 */
		EClass ESTRING = eINSTANCE.getEString();

		/**
		 * The meta object literal for the '{@link oracle.provider.firstRuleIdentity.Choice <em>Choice</em>}' enum.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.firstRuleIdentity.Choice
		 * @see oracle.provider.firstRuleIdentity.impl.FirstRuleIdentityPackageImpl#getChoice()
		 * @generated
		 */
		EEnum CHOICE = eINSTANCE.getChoice();

	}

} //FirstRuleIdentityPackage
