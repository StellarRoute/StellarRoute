import { DemoSwap } from "@/components/DemoSwap";

export default function Home() {
  return (
    <>
      <JsonLd data={faqJsonLd(HOME_FAQS)} />
      <HeroSection />
      <HomeFaqSection />
    </>
  );
}
