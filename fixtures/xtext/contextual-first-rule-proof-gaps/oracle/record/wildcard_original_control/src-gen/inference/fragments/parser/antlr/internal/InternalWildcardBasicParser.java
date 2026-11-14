package inference.fragments.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import inference.fragments.services.WildcardBasicGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalWildcardBasicParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS", "'m'"
    };
    public static final int RULE_ID=4;
    public static final int T__6=6;
    public static final int RULE_WS=5;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalWildcardBasicParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalWildcardBasicParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalWildcardBasicParser.tokenNames; }
    public String getGrammarFileName() { return "InternalWildcardBasic.g"; }



     	private WildcardBasicGrammarAccess grammarAccess;

        public InternalWildcardBasicParser(TokenStream input, WildcardBasicGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected WildcardBasicGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalWildcardBasic.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalWildcardBasic.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalWildcardBasic.g:65:2: iv_ruleModel= ruleModel EOF
            {
             newCompositeNode(grammarAccess.getModelRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleModel=ruleModel();

            state._fsp--;

             current =iv_ruleModel; 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "entryRuleModel"


    // $ANTLR start "ruleModel"
    // InternalWildcardBasic.g:71:1: ruleModel returns [EObject current=null] : (otherlv_0= 'm' this_Fields_1= ruleFields[$current] ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_0=null;
        EObject this_Fields_1 = null;



        	enterRule();

        try {
            // InternalWildcardBasic.g:77:2: ( (otherlv_0= 'm' this_Fields_1= ruleFields[$current] ) )
            // InternalWildcardBasic.g:78:2: (otherlv_0= 'm' this_Fields_1= ruleFields[$current] )
            {
            // InternalWildcardBasic.g:78:2: (otherlv_0= 'm' this_Fields_1= ruleFields[$current] )
            // InternalWildcardBasic.g:79:3: otherlv_0= 'm' this_Fields_1= ruleFields[$current]
            {
            otherlv_0=(Token)match(input,6,FOLLOW_3); 

            			newLeafNode(otherlv_0, grammarAccess.getModelAccess().getMKeyword_0());
            		

            			if (current==null) {
            				current = createModelElement(grammarAccess.getModelRule());
            			}
            			newCompositeNode(grammarAccess.getModelAccess().getFieldsParserRuleCall_1());
            		
            pushFollow(FOLLOW_2);
            this_Fields_1=ruleFields(current);

            state._fsp--;


            			current = this_Fields_1;
            			afterParserOrEnumRuleCall();
            		

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
    // $ANTLR end "ruleModel"


    // $ANTLR start "ruleFields"
    // InternalWildcardBasic.g:99:1: ruleFields[EObject in_current] returns [EObject current=in_current] : ( (lv_name_0_0= RULE_ID ) ) ;
    public final EObject ruleFields(EObject in_current) throws RecognitionException {
        EObject current = in_current;

        Token lv_name_0_0=null;


        	enterRule();

        try {
            // InternalWildcardBasic.g:105:2: ( ( (lv_name_0_0= RULE_ID ) ) )
            // InternalWildcardBasic.g:106:2: ( (lv_name_0_0= RULE_ID ) )
            {
            // InternalWildcardBasic.g:106:2: ( (lv_name_0_0= RULE_ID ) )
            // InternalWildcardBasic.g:107:3: (lv_name_0_0= RULE_ID )
            {
            // InternalWildcardBasic.g:107:3: (lv_name_0_0= RULE_ID )
            // InternalWildcardBasic.g:108:4: lv_name_0_0= RULE_ID
            {
            lv_name_0_0=(Token)match(input,RULE_ID,FOLLOW_2); 

            				newLeafNode(lv_name_0_0, grammarAccess.getFieldsAccess().getNameIDTerminalRuleCall_0());
            			

            				if (current==null) {
            					current = createModelElement(grammarAccess.getFieldsRule());
            				}
            				setWithLastConsumed(
            					current,
            					"name",
            					lv_name_0_0,
            					"inference.fragments.WildcardBasic.ID");
            			

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
    // $ANTLR end "ruleFields"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000010L});

}