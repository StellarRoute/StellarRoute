import { HeroSection } from "@/components/HeroSection";
import { JsonLd } from "@/components/seo/JsonLd";
import { HomeFaqSection } from "@/components/seo/HomeFaqSection";
import {
  DEFAULT_DESCRIPTION,
  DEFAULT_TITLE,
  HOME_FAQS,
  buildPageMetadata,
  faqJsonLd,
} from "@/lib/seo";

export const metadata = buildPageMetadata({
  title: DEFAULT_TITLE,
  description: DEFAULT_DESCRIPTION,
  path: "/",
  absoluteTitle: true,
});

export default function Home() {
  return (
    <>
      <JsonLd data={faqJsonLd(HOME_FAQS)} />
      <HeroSection />
      <HomeFaqSection />
    </>
  );
}
