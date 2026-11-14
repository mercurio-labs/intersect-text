/**
 */
package oracle.provider.colors;

import org.eclipse.emf.ecore.EAttribute;
import org.eclipse.emf.ecore.EClass;
import org.eclipse.emf.ecore.EEnum;
import org.eclipse.emf.ecore.EPackage;

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
 * @see oracle.provider.colors.ColorsFactory
 * @model kind="package"
 * @generated
 */
public interface ColorsPackage extends EPackage {
	/**
	 * The package name.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	String eNAME = "colors";

	/**
	 * The package namespace URI.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	String eNS_URI = "urn:intersect:xtext:enum-composition:1";

	/**
	 * The package namespace name.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	String eNS_PREFIX = "null";

	/**
	 * The singleton instance of the package.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 */
	ColorsPackage eINSTANCE = oracle.provider.colors.impl.ColorsPackageImpl.init();

	/**
	 * The meta object id for the '{@link oracle.provider.colors.impl.ModelImpl <em>Model</em>}' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.colors.impl.ModelImpl
	 * @see oracle.provider.colors.impl.ColorsPackageImpl#getModel()
	 * @generated
	 */
	int MODEL = 0;

	/**
	 * The feature id for the '<em><b>Shade</b></em>' attribute.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int MODEL__SHADE = 0;

	/**
	 * The number of structural features of the '<em>Model</em>' class.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @generated
	 * @ordered
	 */
	int MODEL_FEATURE_COUNT = 1;

	/**
	 * The meta object id for the '{@link oracle.provider.colors.Shade <em>Shade</em>}' enum.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.colors.Shade
	 * @see oracle.provider.colors.impl.ColorsPackageImpl#getShade()
	 * @generated
	 */
	int SHADE = 1;

	/**
	 * The meta object id for the '{@link oracle.provider.colors.OtherShade <em>Other Shade</em>}' enum.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @see oracle.provider.colors.OtherShade
	 * @see oracle.provider.colors.impl.ColorsPackageImpl#getOtherShade()
	 * @generated
	 */
	int OTHER_SHADE = 2;


	/**
	 * Returns the meta object for class '{@link oracle.provider.colors.Model <em>Model</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for class '<em>Model</em>'.
	 * @see oracle.provider.colors.Model
	 * @generated
	 */
	EClass getModel();

	/**
	 * Returns the meta object for the attribute '{@link oracle.provider.colors.Model#getShade <em>Shade</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for the attribute '<em>Shade</em>'.
	 * @see oracle.provider.colors.Model#getShade()
	 * @see #getModel()
	 * @generated
	 */
	EAttribute getModel_Shade();

	/**
	 * Returns the meta object for enum '{@link oracle.provider.colors.Shade <em>Shade</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for enum '<em>Shade</em>'.
	 * @see oracle.provider.colors.Shade
	 * @generated
	 */
	EEnum getShade();

	/**
	 * Returns the meta object for enum '{@link oracle.provider.colors.OtherShade <em>Other Shade</em>}'.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the meta object for enum '<em>Other Shade</em>'.
	 * @see oracle.provider.colors.OtherShade
	 * @generated
	 */
	EEnum getOtherShade();

	/**
	 * Returns the factory that creates the instances of the model.
	 * <!-- begin-user-doc -->
	 * <!-- end-user-doc -->
	 * @return the factory that creates the instances of the model.
	 * @generated
	 */
	ColorsFactory getColorsFactory();

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
		 * The meta object literal for the '{@link oracle.provider.colors.impl.ModelImpl <em>Model</em>}' class.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.colors.impl.ModelImpl
		 * @see oracle.provider.colors.impl.ColorsPackageImpl#getModel()
		 * @generated
		 */
		EClass MODEL = eINSTANCE.getModel();

		/**
		 * The meta object literal for the '<em><b>Shade</b></em>' attribute feature.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @generated
		 */
		EAttribute MODEL__SHADE = eINSTANCE.getModel_Shade();

		/**
		 * The meta object literal for the '{@link oracle.provider.colors.Shade <em>Shade</em>}' enum.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.colors.Shade
		 * @see oracle.provider.colors.impl.ColorsPackageImpl#getShade()
		 * @generated
		 */
		EEnum SHADE = eINSTANCE.getShade();

		/**
		 * The meta object literal for the '{@link oracle.provider.colors.OtherShade <em>Other Shade</em>}' enum.
		 * <!-- begin-user-doc -->
		 * <!-- end-user-doc -->
		 * @see oracle.provider.colors.OtherShade
		 * @see oracle.provider.colors.impl.ColorsPackageImpl#getOtherShade()
		 * @generated
		 */
		EEnum OTHER_SHADE = eINSTANCE.getOtherShade();

	}

} //ColorsPackage
