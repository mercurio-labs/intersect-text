package audit.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import audit.services.FirstRuleClassFragmentGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalFirstRuleClassFragmentParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_VALUE"
    };
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalFirstRuleClassFragmentParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalFirstRuleClassFragmentParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalFirstRuleClassFragmentParser.tokenNames; }
    public String getGrammarFileName() { return "InternalFirstRuleClassFragment.g"; }



     	private FirstRuleClassFragmentGrammarAccess grammarAccess;

        public InternalFirstRuleClassFragmentParser(TokenStream input, FirstRuleClassFragmentGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "First";
       	}

       	@Override
       	protected FirstRuleClassFragmentGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "ruleFirst"
    // InternalFirstRuleClassFragment.g:65:1: ruleFirst[EObject in_current] returns [EObject current=in_current] : ( (lv_name_0_0= RULE_VALUE ) ) ;
    public final EObject ruleFirst(EObject in_current) throws RecognitionException {
        EObject current = in_current;

        Token lv_name_0_0=null;


        	enterRule();

        try {
            // InternalFirstRuleClassFragment.g:71:2: ( ( (lv_name_0_0= RULE_VALUE ) ) )
            // InternalFirstRuleClassFragment.g:72:2: ( (lv_name_0_0= RULE_VALUE ) )
            {
            // InternalFirstRuleClassFragment.g:72:2: ( (lv_name_0_0= RULE_VALUE ) )
            // InternalFirstRuleClassFragment.g:73:3: (lv_name_0_0= RULE_VALUE )
            {
            // InternalFirstRuleClassFragment.g:73:3: (lv_name_0_0= RULE_VALUE )
            // InternalFirstRuleClassFragment.g:74:4: lv_name_0_0= RULE_VALUE
            {
            lv_name_0_0=(Token)match(input,RULE_VALUE,FOLLOW_1); 

            				newLeafNode(lv_name_0_0, grammarAccess.getFirstAccess().getNameVALUETerminalRuleCall_0());
            			

            				if (current==null) {
            					current = createModelElement(grammarAccess.getFirstRule());
            				}
            				setWithLastConsumed(
            					current,
            					"name",
            					lv_name_0_0,
            					"audit.FirstRuleClassFragment.VALUE");
            			

            }


            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "ruleFirst"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000002L});

}