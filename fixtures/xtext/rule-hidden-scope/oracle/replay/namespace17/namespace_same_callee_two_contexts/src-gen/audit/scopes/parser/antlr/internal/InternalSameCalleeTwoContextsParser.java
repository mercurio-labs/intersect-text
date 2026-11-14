package audit.scopes.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import audit.scopes.services.SameCalleeTwoContextsGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalSameCalleeTwoContextsParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS", "RULE_ML_COMMENT", "RULE_SL_COMMENT", "'loose'", "'tight'", "'a'", "'b'"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int RULE_ID=4;
    public static final int RULE_WS=5;
    public static final int RULE_SL_COMMENT=7;
    public static final int T__11=11;
    public static final int RULE_ML_COMMENT=6;
    public static final int EOF=-1;
    public static final int T__10=10;

    // delegates
    // delegators


        public InternalSameCalleeTwoContextsParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalSameCalleeTwoContextsParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalSameCalleeTwoContextsParser.tokenNames; }
    public String getGrammarFileName() { return "InternalSameCalleeTwoContexts.g"; }



     	private SameCalleeTwoContextsGrammarAccess grammarAccess;

        public InternalSameCalleeTwoContextsParser(TokenStream input, SameCalleeTwoContextsGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected SameCalleeTwoContextsGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalSameCalleeTwoContexts.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalSameCalleeTwoContexts.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalSameCalleeTwoContexts.g:65:2: iv_ruleModel= ruleModel EOF
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
    // InternalSameCalleeTwoContexts.g:71:1: ruleModel returns [EObject current=null] : ( (otherlv_0= 'loose' ( (lv_name_1_0= ruleLoose ) ) ) | (otherlv_2= 'tight' ( (lv_name_3_0= ruleTight ) ) ) ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_0=null;
        Token otherlv_2=null;
        AntlrDatatypeRuleToken lv_name_1_0 = null;

        AntlrDatatypeRuleToken lv_name_3_0 = null;



        	enterRule();

        try {
            // InternalSameCalleeTwoContexts.g:77:2: ( ( (otherlv_0= 'loose' ( (lv_name_1_0= ruleLoose ) ) ) | (otherlv_2= 'tight' ( (lv_name_3_0= ruleTight ) ) ) ) )
            // InternalSameCalleeTwoContexts.g:78:2: ( (otherlv_0= 'loose' ( (lv_name_1_0= ruleLoose ) ) ) | (otherlv_2= 'tight' ( (lv_name_3_0= ruleTight ) ) ) )
            {
            // InternalSameCalleeTwoContexts.g:78:2: ( (otherlv_0= 'loose' ( (lv_name_1_0= ruleLoose ) ) ) | (otherlv_2= 'tight' ( (lv_name_3_0= ruleTight ) ) ) )
            int alt1=2;
            int LA1_0 = input.LA(1);

            if ( (LA1_0==8) ) {
                alt1=1;
            }
            else if ( (LA1_0==9) ) {
                alt1=2;
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 1, 0, input);

                throw nvae;
            }
            switch (alt1) {
                case 1 :
                    // InternalSameCalleeTwoContexts.g:79:3: (otherlv_0= 'loose' ( (lv_name_1_0= ruleLoose ) ) )
                    {
                    // InternalSameCalleeTwoContexts.g:79:3: (otherlv_0= 'loose' ( (lv_name_1_0= ruleLoose ) ) )
                    // InternalSameCalleeTwoContexts.g:80:4: otherlv_0= 'loose' ( (lv_name_1_0= ruleLoose ) )
                    {
                    otherlv_0=(Token)match(input,8,FOLLOW_3); 

                    				newLeafNode(otherlv_0, grammarAccess.getModelAccess().getLooseKeyword_0_0());
                    			
                    // InternalSameCalleeTwoContexts.g:84:4: ( (lv_name_1_0= ruleLoose ) )
                    // InternalSameCalleeTwoContexts.g:85:5: (lv_name_1_0= ruleLoose )
                    {
                    // InternalSameCalleeTwoContexts.g:85:5: (lv_name_1_0= ruleLoose )
                    // InternalSameCalleeTwoContexts.g:86:6: lv_name_1_0= ruleLoose
                    {

                    						newCompositeNode(grammarAccess.getModelAccess().getNameLooseParserRuleCall_0_1_0());
                    					
                    pushFollow(FOLLOW_2);
                    lv_name_1_0=ruleLoose();

                    state._fsp--;


                    						if (current==null) {
                    							current = createModelElementForParent(grammarAccess.getModelRule());
                    						}
                    						set(
                    							current,
                    							"name",
                    							lv_name_1_0,
                    							"audit.scopes.SameCalleeTwoContexts.Loose");
                    						afterParserOrEnumRuleCall();
                    					

                    }


                    }


                    }


                    }
                    break;
                case 2 :
                    // InternalSameCalleeTwoContexts.g:105:3: (otherlv_2= 'tight' ( (lv_name_3_0= ruleTight ) ) )
                    {
                    // InternalSameCalleeTwoContexts.g:105:3: (otherlv_2= 'tight' ( (lv_name_3_0= ruleTight ) ) )
                    // InternalSameCalleeTwoContexts.g:106:4: otherlv_2= 'tight' ( (lv_name_3_0= ruleTight ) )
                    {
                    otherlv_2=(Token)match(input,9,FOLLOW_3); 

                    				newLeafNode(otherlv_2, grammarAccess.getModelAccess().getTightKeyword_1_0());
                    			
                    // InternalSameCalleeTwoContexts.g:110:4: ( (lv_name_3_0= ruleTight ) )
                    // InternalSameCalleeTwoContexts.g:111:5: (lv_name_3_0= ruleTight )
                    {
                    // InternalSameCalleeTwoContexts.g:111:5: (lv_name_3_0= ruleTight )
                    // InternalSameCalleeTwoContexts.g:112:6: lv_name_3_0= ruleTight
                    {

                    						newCompositeNode(grammarAccess.getModelAccess().getNameTightParserRuleCall_1_1_0());
                    					
                    pushFollow(FOLLOW_2);
                    lv_name_3_0=ruleTight();

                    state._fsp--;


                    						if (current==null) {
                    							current = createModelElementForParent(grammarAccess.getModelRule());
                    						}
                    						set(
                    							current,
                    							"name",
                    							lv_name_3_0,
                    							"audit.scopes.SameCalleeTwoContexts.Tight");
                    						afterParserOrEnumRuleCall();
                    					

                    }


                    }


                    }


                    }
                    break;

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


    // $ANTLR start "entryRuleLoose"
    // InternalSameCalleeTwoContexts.g:134:1: entryRuleLoose returns [String current=null] : iv_ruleLoose= ruleLoose EOF ;
    public final String entryRuleLoose() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleLoose = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalSameCalleeTwoContexts.g:136:2: (iv_ruleLoose= ruleLoose EOF )
            // InternalSameCalleeTwoContexts.g:137:2: iv_ruleLoose= ruleLoose EOF
            {
             newCompositeNode(grammarAccess.getLooseRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleLoose=ruleLoose();

            state._fsp--;

             current =iv_ruleLoose.getText(); 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "entryRuleLoose"


    // $ANTLR start "ruleLoose"
    // InternalSameCalleeTwoContexts.g:146:1: ruleLoose returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : this_Word_0= ruleWord ;
    public final AntlrDatatypeRuleToken ruleLoose() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        AntlrDatatypeRuleToken this_Word_0 = null;



        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens("RULE_WS", "RULE_ML_COMMENT");

        try {
            // InternalSameCalleeTwoContexts.g:153:2: (this_Word_0= ruleWord )
            // InternalSameCalleeTwoContexts.g:154:2: this_Word_0= ruleWord
            {

            		newCompositeNode(grammarAccess.getLooseAccess().getWordParserRuleCall());
            	
            pushFollow(FOLLOW_2);
            this_Word_0=ruleWord();

            state._fsp--;


            		current.merge(this_Word_0);
            	

            		afterParserOrEnumRuleCall();
            	

            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "ruleLoose"


    // $ANTLR start "entryRuleTight"
    // InternalSameCalleeTwoContexts.g:170:1: entryRuleTight returns [String current=null] : iv_ruleTight= ruleTight EOF ;
    public final String entryRuleTight() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleTight = null;



        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalSameCalleeTwoContexts.g:172:2: (iv_ruleTight= ruleTight EOF )
            // InternalSameCalleeTwoContexts.g:173:2: iv_ruleTight= ruleTight EOF
            {
             newCompositeNode(grammarAccess.getTightRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleTight=ruleTight();

            state._fsp--;

             current =iv_ruleTight.getText(); 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "entryRuleTight"


    // $ANTLR start "ruleTight"
    // InternalSameCalleeTwoContexts.g:182:1: ruleTight returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : this_Word_0= ruleWord ;
    public final AntlrDatatypeRuleToken ruleTight() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        AntlrDatatypeRuleToken this_Word_0 = null;



        	enterRule();
        	HiddenTokens myHiddenTokenState = ((XtextTokenStream)input).setHiddenTokens();

        try {
            // InternalSameCalleeTwoContexts.g:189:2: (this_Word_0= ruleWord )
            // InternalSameCalleeTwoContexts.g:190:2: this_Word_0= ruleWord
            {

            		newCompositeNode(grammarAccess.getTightAccess().getWordParserRuleCall());
            	
            pushFollow(FOLLOW_2);
            this_Word_0=ruleWord();

            state._fsp--;


            		current.merge(this_Word_0);
            	

            		afterParserOrEnumRuleCall();
            	

            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {

            	myHiddenTokenState.restore();

        }
        return current;
    }
    // $ANTLR end "ruleTight"


    // $ANTLR start "entryRuleWord"
    // InternalSameCalleeTwoContexts.g:206:1: entryRuleWord returns [String current=null] : iv_ruleWord= ruleWord EOF ;
    public final String entryRuleWord() throws RecognitionException {
        String current = null;

        AntlrDatatypeRuleToken iv_ruleWord = null;


        try {
            // InternalSameCalleeTwoContexts.g:206:44: (iv_ruleWord= ruleWord EOF )
            // InternalSameCalleeTwoContexts.g:207:2: iv_ruleWord= ruleWord EOF
            {
             newCompositeNode(grammarAccess.getWordRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleWord=ruleWord();

            state._fsp--;

             current =iv_ruleWord.getText(); 
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
    // $ANTLR end "entryRuleWord"


    // $ANTLR start "ruleWord"
    // InternalSameCalleeTwoContexts.g:213:1: ruleWord returns [AntlrDatatypeRuleToken current=new AntlrDatatypeRuleToken()] : (kw= 'a' kw= 'b' ) ;
    public final AntlrDatatypeRuleToken ruleWord() throws RecognitionException {
        AntlrDatatypeRuleToken current = new AntlrDatatypeRuleToken();

        Token kw=null;


        	enterRule();

        try {
            // InternalSameCalleeTwoContexts.g:219:2: ( (kw= 'a' kw= 'b' ) )
            // InternalSameCalleeTwoContexts.g:220:2: (kw= 'a' kw= 'b' )
            {
            // InternalSameCalleeTwoContexts.g:220:2: (kw= 'a' kw= 'b' )
            // InternalSameCalleeTwoContexts.g:221:3: kw= 'a' kw= 'b'
            {
            kw=(Token)match(input,10,FOLLOW_4); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getWordAccess().getAKeyword_0());
            		
            kw=(Token)match(input,11,FOLLOW_2); 

            			current.merge(kw);
            			newLeafNode(kw, grammarAccess.getWordAccess().getBKeyword_1());
            		

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
    // $ANTLR end "ruleWord"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000400L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000800L});

}